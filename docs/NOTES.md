# Styky — implementation notes

Running log of deviations from `PLAN.md`, findings and known limitations.

## Toolchain (M0)

- Rust 1.99 (`stable-x86_64-pc-windows-msvc`) via `winget Rustlang.Rustup`; VS 2022 Build Tools (VCTools workload) via winget.
  The VS installer reports "Restart your PC to finish installation", but `link.exe` works without a reboot.
- A fresh shell does not have `%USERPROFILE%\.cargo\bin` on `PATH` until the next login; prepend it by hand (`$env:Path += ";$env:USERPROFILE\.cargo\bin"`).
- `create-tauri-app` was run in a scratch directory and copied in, because `D:\dev\styky` already contained `docs/`.
- Scaffold versions: Tauri 2.12, React 19 (plan said 18; the template's 19 is current), Vite 8, TypeScript 6. `edition = "2024"`.
- `productName` is `Styky`, identifier `com.styky.app`.

## Deviations / decisions (M1)

1. **Acrylic is the default material, Mica is the fallback** (plan: Mica first). Mica only renders translucently while its
   window is *active*; an inactive window gets a flat fallback colour, and a desktop-layer widget is inactive almost always.
   Acrylic keeps its blur when inactive. `STYKY_MATERIAL=mica|acrylic|none` overrides it for experiments. Acrylic tint alpha is 0
   so only the CSS tint (`--panel-tint`, per the plan's rgba values) colours the panel. Look on very dark wallpapers is close to
   flat grey; it should be re-checked by eye on a lighter wallpaper (see "Not verified").
2. **`set_always_on_bottom` / `HWND_BOTTOM` is not used.** HWND_BOTTOM puts the panel *under* Progman (the desktop), so the
   panel disappears whenever the desktop is shown. `src-tauri/src/desktop.rs` instead inserts the panel directly above Progman
   (below all normal windows). It is re-applied on `Focused(false)`.
3. **Window-state saving.** The plugin does not save when we `prevent_close()`, so we save explicitly on close-to-hide and
   (debounced 600 ms) on every move/resize. `VISIBLE` and `DECORATIONS` are excluded from restore (the app always shows at
   start; it is frameless). First run places the panel top-right with a 24 px margin of the primary work area.
4. Tray menu has an extra **Launch at login** check item (wired to the autostart commands) so the toggle is usable before the
   Settings window exists (M4). Header `+` and the `⋯ → Settings…` entry are present but disabled placeholders until M2/M4.
5. UI uses CSS transitions/keyframes only for now (no `motion` dependency yet); the spring pass is M4. Added deps:
   `@fontsource-variable/inter`, `lucide-react`.
6. `--accent` is read from Rust (`UISettings.GetColorValue(Accent)`, command `get_accent`) at start and updated live via the
   `accent-changed` event (WinRT `ColorValuesChanged`). Light/dark follow `prefers-color-scheme`, which WebView2 tracks live.

## Win+D / Show Desktop investigation (time-boxed; ~1h used)

**Findings**

- Win+D does **not** just minimise windows: Windows also makes the desktop (`Progman`) a *topmost* window for as long as the
  desktop is shown. Anything at the bottom of the normal z-order, including a window placed just above Progman, is covered.
- A `skipTaskbar` panel (WS_EX_TOOLWINDOW) is **not** minimised by Win+D (it stays `IsWindowVisible` and non-iconic), so it is
  only the z-order that hides it.
- `HWND_BOTTOM` is worse: the panel is under the desktop and invisible even when no windows are open.

**Solution that shipped (robust, input still works — it is a normal top-level window, no reparenting):**
`desktop.rs` runs a 200 ms watcher thread. If no ordinary app window is visible (visible, not minimised, not cloaked, not a
tool window, not owned, titled, ≥ 80×80, not shell classes, not our own process) it makes the panel `HWND_TOPMOST`, which lands
above the desktop; as soon as an app window is visible again it drops back to "directly above Progman". Verified with
synthesized Win+D key presses and z-order dumps (`scripts/zorder.ps1`, `scripts/windtest.ps1`):

| state | z-order |
|---|---|
| apps visible | panel just above Progman (below all apps) |
| Win+D (desktop shown) | panel topmost, above Progman; screenshot confirms it is painted |
| Win+D again | back to just above Progman |

**Rejected:** WorkerW/Progman reparenting (`SetParent`) — a child window cannot use the DWM acrylic/Mica backdrop and is
awkward for WebView2; not worth losing the glass.

**Known limitations**

- Detection is polling (200 ms), so after Win+D the panel can take up to ~0.2 s to reappear.
- With *no* app windows open at all the panel is topmost (harmless, nothing to cover). A *topmost* panel would also sit above the
  taskbar if the user drags it over it, only while the desktop is shown.
- Other "show desktop"-like states (e.g. all windows minimised by hand) behave the same as Win+D: the panel is raised. That is
  the intended behaviour.
- A full-screen-exclusive app/game hides the panel like any window; not handled.

## Not verified (could not be checked from this session)

- Light theme / live light↔dark switching: not exercised (changing the user's Windows theme was out of bounds). The tokens exist
  in both modes and `prefers-color-scheme` is the mechanism; only dark was seen on screen.
- Mica fallback path and Windows accent *change* event (initial accent read was not eyeballed; the purple accent seen in the
  empty-state icon matches this machine's accent).
- Tray: left-click toggle, menu items, "Launch at login" toggle, the autostart registry entry (code compiles; not clicked).
- Header drag region and edge-resize with the mouse, hover/press states (only static screenshots were taken).
- Behaviour at display scales other than 100 % / on multi-monitor.
- Close-to-hide, single-instance (second launch re-shows the hidden panel) and restart persistence **were** verified by
  script (window message + relaunch + rect query).

## M2 notes (core list)

Deviations / decisions:
- **Snooze UI and the `S` key are not in M2** (they belong with the M3 SnoozeMenu). The `snooze_item` command exists and is tested.
- **Monthly repeat drift:** within one catch-up the day-of-month is anchored (31st stays 31st), but across completions there is no
  stored anchor (schema is fixed by the plan), so Jan 31 -> Feb 28 -> Mar 28. Fix would need an extra column (migration v2).
- Drag-reorder applies to the trailing group of plain items (no due, not urgent), the only ones ordered by `sort_order`.
- `react-markdown` runs through `rehype-sanitize` (GitHub schema plus `checked`); task boxes are our own buttons that call
  `toggleTaskLine(source, line)` using the `li` node position, so exactly one source line is rewritten (vitest-covered).
- Due picker is the native `datetime-local` input (WebView2 popup). Overdue/soon colours use darker red/orange in light mode for text contrast.
- `Esc` in the editor reverts to the values at edit start (autosave may already have written some); `Ctrl+Enter` saves and closes.
  A new item is created on the first save that has a title, and hidden from the list while its draft card is open.
- Export file = `{version:1, exportedAt, items[], settings{}}` (camelCase). Merge keeps the newer `updated_at` per id; replace wipes items and settings.

Not verified by hand: light theme, mouse drag-reorder, the native file dialogs behind Export/Import (the underlying commands were
round-tripped live), and real keyboard hardware (keys were injected via CDP). CRUD, reorder, repeat roll-forward and export/replace-import
were verified against the running app, including across a restart (see `scripts/cdp.mjs`).

## Handoff (for the next implementor)

**Rust modules** (`src-tauri/src/`)
- `lib.rs` builder, plugins, setup, window events (close-to-hide, debounced state save) / `commands.rs` every `#[tauri::command]`
- `db.rs` rusqlite store, `user_version` migrations, CRUD/complete/snooze/reorder/purge/export/import (+ tests) / `model.rs` Item, ItemEdit, ExportFile
- `sort.rs` list order (+ tests) / `repeat.rs` next-occurrence (+ tests) / `window.rs` material, positioning, show/hide
- `desktop.rs` z-order + Show-Desktop watcher / `theme.rs` Windows accent + watcher / `tray.rs` tray icon and menu

**React** (`src/`): `app/Panel.tsx` shell, header menu, keyboard, draft card; `components/ItemRow.tsx` row + expand; `Editor.tsx` autosave editor;
`MarkdownView.tsx`; `PlainGroup.tsx` reorder; `CompletedSection.tsx`; `Toast.tsx`; `lib/store.ts` zustand (all UI state, undo toast, check-then-complete);
`lib/api.ts` typed invoke wrappers; `lib/time.ts`, `tags.ts`, `markdown.ts`, `motion.ts` (springs); `styles/tokens.css` design tokens.

**API.** Commands: `list_items, create_item(edit), update_item(id,edit), delete_item, undelete_item(item), complete_item, restore_item, snooze_item(id,until),
reorder(ids), get_settings, set_setting(key,value), export_json(path), import_json(path,mode), get_accent, get_autostart, set_autostart, hide_panel, quit`.
Events: `items-changed` after every mutation, `settings-changed`, `accent-changed` (`#rrggbb`), `open-settings`. Items are camelCase JSON; times are unix ms UTC.
Settings: only `autoclear_days` (default 7, 0 = keep). Completed items are purged lazily inside `list_items`.

**Dev workflow**
- New shells lack cargo: `$env:Path += ";$env:USERPROFILE\.cargo\bin"` (PowerShell) / `export PATH="$PATH:$HOME/.cargo/bin"` (bash).
- Run: `pnpm tauri dev` (Rust and Vite hot-reload; touching any `src-tauri/src/*.rs` restarts the app). Test: `cargo test` (in `src-tauri`), `pnpm test` (vitest).
  Lint: `cargo clippy --all-targets`, `pnpm tsc --noEmit`. All clean at the end of M2.
- Drive the real webview: start dev with `$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"`, then
  `node scripts/cdp.mjs eval "<js>"` / `shot out.png` / `key ArrowDown`. `scripts/seed.js` and `scripts/set.js` are paste-in helpers (seed data, set React inputs).
  A CDP screenshot has no acrylic behind it (looks flat grey); the real window is translucent.
- Win32 helpers: `winpos.ps1` (panel rect), `zorder.ps1 [-Top]`, `windtest.ps1` (synthetic Win+D with screenshots), `moveshot.ps1`.
- Windows only: **no real Python** (`python3` is the Store stub and can hang). Script with Node or PowerShell. In bash, apostrophes inside heredocs
  have broken tool calls; prefer the Write/Edit tools for files. Never broad-kill `node`/`cargo`; stop only the dev task you started.
- The DB is `%APPDATA%\com.styky.app\styky.db`; the window state is `%APPDATA%\com.styky.app\.window-state.json`.

**Gotchas**
- **Z-order (`desktop.rs`)**: never use `HWND_BOTTOM`/`set_always_on_bottom` (hides the panel under Progman). The watcher thread (200 ms) makes the panel
  `TOPMOST` while no ordinary app window is visible (Win+D) and drops it to just above Progman otherwise. Any new window of ours that is topmost
  (quick-add, toast host, settings) must call `desktop::suspend(true)` while shown and `suspend(false)` when hidden, or the watcher will fight it
  (`suspend` exists but is unused so far; it is marked `#[allow(dead_code)]`).
- **Window state**: the plugin does not save when we `prevent_close`, so `lib.rs` saves explicitly on hide and (debounced) on move/resize.
  `VISIBLE` and `DECORATIONS` are excluded from restore; first run (no state file) places the panel top-right.
- **Material**: Acrylic first, Mica fallback (Mica goes flat when the window is inactive, which is nearly always for this widget).
- `windows` crate 0.61: `BOOL` is `windows::core::BOOL`. Edition 2024 (let-chains are used).
- Tauri permissions live in `src-tauri/capabilities/default.json`; the quick-add/settings windows will need their own capability entries.

**Stubbed for later**
- M3: scheduler (`last_fired_at` is stored but unused), toasts, snooze UI + `S` key, quick-add window + `Ctrl+Alt+N`, NL parser, `desktop::suspend` wiring.
- M4: Settings window (header and tray `Settings...` are placeholders; tray emits `open-settings`), updater, installer, icon, motion polish, README.
- Header `⋯` menu lacks nothing from the plan except Settings; `autoclear_days` has no UI yet.

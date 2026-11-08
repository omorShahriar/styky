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
  stored anchor (schema is fixed by the plan), so Jan 31 -> Feb 28 -> Mar 28. Fixed in M3 (migration v2, `repeat_anchor_day`).
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

## M3 notes (reminders and capture)

Deviations / decisions:
- **Toasts do not use `tauri-winrt-notification`.** v0.8.1 only has buttons: no `<input type="selection">`, and `on_activated` hands back only
  the button arguments, never the dropdown value. `notify.rs` builds the toast XML itself and shows it through the `windows` crate
  (features `UI_Notifications`, `Data_Xml_Dom`, `Foundation_Collections`); the crate dependency was dropped. Toast: title, "Due Today 17:00",
  "#tag", `scenario="reminder"` (stays until dismissed), a Snooze dropdown (10 min, 1 hour, This evening 18:00, Tomorrow 09:00, Custom...) + **Snooze**
  and **Done** buttons. Body click = open the panel at the item. **Custom...** opens the panel with the custom picker.
- **AUMID**: Tauri's NSIS template stamps its Start-menu/desktop shortcuts with `BUNDLEID` (read from the CLI binary), so a release build uses
  `com.styky.app`. In `tauri dev` (`tauri::is_dev()`) the PowerShell AUMID is used and the toast says "Windows PowerShell".
- Activation callbacks are in-process only. A toast left in Action Center after Styky quits does nothing when clicked (Windows may launch
  the app with the toast arguments; not handled).
- **Scheduler** (`scheduler.rs`): a pass reads the wall clock, fires every open item with `effective_at <= now` and `last_fired_at < effective_at`
  (marks first, then toasts), sleeps min(next due, 30 s) and wakes on `items-changed`. "Catch-up" mode (app start, a >90 s wall-clock gap =
  sleep/resume, clock set back, end of a pause) summarises items due more than 60 s ago in ONE toast ("3 reminders were due") when there are two
  or more; a single missed one is a normal toast. A repeating item that was ignored is not advanced; it stays overdue until completed.
- **Pause reminders 1h**: tray item toggles to "Resume reminders (paused until HH:MM)". In-memory only (resets on restart).
- **Snooze presets** (`snooze.rs`, `lib/time.ts`): "This evening" = today 18:00, or tomorrow 18:00 once it is past 18:00.
- **Quick-add**: `quickadd` window declared in `tauri.conf.json` (560x64, grows to 100 high when chips show), own capability `capabilities/quickadd.json`.
  Hotkey from setting `quickadd_hotkey` (default `Ctrl+Alt+N`, now in `DEFAULT_SETTINGS`); `set_quickadd_hotkey` command rebinds it (for the M4 recorder).
  It calls `desktop::suspend(true/false)`; shown centred horizontally at 28 % height on the monitor under the cursor. Blur, Esc and Enter hide it.
  Tray has a **Quick add** item.
- **NL grammar** (`lib/nlp.ts`, documented at the top): `!!` urgent (2), `!` explicit normal (1, shows a "Normal" chip), `!low` low (0); `#tag`; `@branch`;
  URL -> `link`; `every day|weekday|week|month`; any chrono-node date. Date without a time = 09:00; repeat without a date = next 09:00 (skipping
  the weekend for weekdays). `@`/`#` must start a word, so e-mail addresses and "C#" are left alone.
- Snooze UI: one popover (`SnoozeMenu`, fixed-positioned so the list never clips it) opened from the row hover clock, the expanded **Snooze** button,
  the `S` key, or a toast's Custom... (`focus-item` event, `{id, snooze}`). Items now carry `repeatAnchorDay` (null unless monthly).
- Export version is still 1; `repeatAnchorDay` is `serde(default)` so older exports import fine.

Verified against the running dev build (CDP + synthesized keys): `Ctrl+Alt+N` shows the bar focused; chips for
"Rotate keys fri 10am #infra every month !!" are right; Enter creates the item (monthly, anchor 13) and hides; Esc hides; the snooze popover lists the presets and
snoozes. The scheduler fired a test item 10 ms after its due time and wrote `last_fired_at`; Windows toast history for the dev AUMID confirms the toast was delivered
(`scripts/opennc.ps1`, `toastclick.ps1` are the helpers I used).

**Not verified**: clicking **Done**, **Snooze** (dropdown) or the body on a real toast, and seeing the toast on screen. Screen access was denied for this session
and the banner/Action Center did not appear in UI Automation, so only delivery to the toast history was confirmed. The parsing of the activation
(`parse_action`) and the DB effects are unit-tested. Also not checked: sleep/resume catch-up and the startup summary toast on a real machine
(logic unit-tested), the installed-build AUMID, light theme for the new UI, hotkey conflicts (a failed registration only logs to stderr).

## Handoff (for the next implementor, M4)

**Rust modules** (`src-tauri/src/`)
- `lib.rs` builder, plugins, setup, window events / `commands.rs` every `#[tauri::command]` / `db.rs` rusqlite store, migrations v1+v2, CRUD, `list_open`, `mark_fired` / `model.rs` Item, ItemEdit, ExportFile
- `sort.rs` / `repeat.rs` (next occurrence, monthly anchor) / `snooze.rs` presets / `scheduler.rs` due engine + pause / `notify.rs` WinRT toasts + activation / `quickadd.rs` window + hotkey
- `window.rs` material, positioning, show/hide / `desktop.rs` z-order + Show-Desktop watcher + `suspend` / `theme.rs` accent / `tray.rs` tray menu

**React** (`src/`): `main.tsx` picks the app by window label (`quickadd` -> `app/QuickAdd.tsx`, else `app/Panel.tsx`); `components/ItemRow.tsx`, `Editor.tsx`, `MarkdownView.tsx`,
`SnoozeMenu.tsx`, `PlainGroup.tsx`, `CompletedSection.tsx`, `Toast.tsx`; `lib/store.ts` zustand; `api.ts` invoke wrappers; `nlp.ts` quick-add parser; `time.ts`; `styles/tokens.css`.

**API.** Commands: `list_items, create_item(edit), update_item(id,edit), delete_item, undelete_item(item), complete_item, restore_item, snooze_item(id,until),
reorder(ids), get_settings, set_setting(key,value), export_json(path), import_json(path,mode), get_accent, get_autostart, set_autostart, hide_panel, hide_quickadd,
set_quickadd_hotkey(hotkey), quit`. Events: `items-changed`, `settings-changed`, `accent-changed`, `open-settings`, `focus-item` ({id, snooze}), `quickadd-shown` (to the quickadd window).
Items are camelCase JSON, times unix ms UTC. Settings: `autoclear_days` (7), `quickadd_hotkey` ("Ctrl+Alt+N"). M4 still needs UIs for both, "default snooze", theme.

**Dev workflow**
- New shells lack cargo: `$env:Path += ";$env:USERPROFILE\.cargo\bin"` / bash `export PATH="$PATH:$HOME/.cargo/bin"`. Run `pnpm tauri dev` (touching `src-tauri/src/*.rs` restarts the app).
  Test `cargo test` (in `src-tauri`), `pnpm test`; lint `cargo clippy --all-targets`, `pnpm tsc --noEmit`. All clean at the end of M3 (34 Rust, 17 vitest).
- Drive the webview: start dev with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`. `scripts/cdp.mjs` (first page) or
  `scripts/cdpx.mjs <label> "<js>"` (page of a given window label, e.g. `main`, `quickadd`; use `window.__TAURI_INTERNALS__.invoke` for commands).
  `scripts/hotkey.ps1 ctrl+alt+n|enter|escape` synthesizes keys; `opennc.ps1` / `toastclick.ps1` poke the notification center. Win32: `winpos.ps1`, `zorder.ps1`, `windtest.ps1`.
- Windows only: no real Python. Many repo files are CRLF, so scripted multi-line edits must normalise line endings. Apostrophes in bash heredocs / `node -e` break tool calls: use the Write/Edit tools. Never broad-kill `node`/`cargo`.
- DB `%APPDATA%\com.styky.app\styky.db`; window state `.window-state.json` next to it. Test items you create in dev land in the real dev DB; delete them.

**Gotchas**
- **Z-order (`desktop.rs`)**: never `HWND_BOTTOM`/`set_always_on_bottom`. The 200 ms watcher makes the panel TOPMOST while no ordinary app window is visible. Any of our own topmost windows must call
  `desktop::suspend(true/false)` (quick-add does; Settings must too).
- **Toasts** are hand-built XML (see M3 notes); AUMID is `com.styky.app` in release builds only. Verify a toast from the *installed* build in M4.
- **Window state**: the plugin does not save on `prevent_close`; `lib.rs` saves explicitly. `VISIBLE`/`DECORATIONS` are excluded from restore.
- **Material**: Acrylic first, Mica fallback; `quickadd` uses the same `apply_material`. `windows` crate 0.61 (`BOOL` is `windows::core::BOOL`), edition 2024.
- Tauri permissions are per window in `capabilities/` (`default.json` = main, `quickadd.json`); the Settings window needs its own entry.
- Hotkey registration failure (key taken) only logs to stderr; the Settings recorder should surface the error from `set_quickadd_hotkey`.

**Stubbed for M4**
- Settings window (header and tray `Settings...` are placeholders; tray emits `open-settings`): theme, hotkey recorder, autostart, autoclear, default snooze, export/import, updater/about.
- Updater, NSIS installer, app icon, motion polish, README (hotkeys + Win+D note). Pause state is not persisted and has no panel indicator.

## M4 notes (polish and ship)

Deviations / decisions:
- **Settings window** (`settings` label, `capabilities/settings.json`, `settings_win.rs`) is a normal *decorated, opaque* window (480x660, hidden on close),
  not a glass panel: grouped inset cards on `--win-bg`/`--group-bg` tokens. It is not topmost, so it does **not** call `desktop::suspend`
  (the Show-Desktop watcher sees it as an ordinary app window). `open_settings` command + tray item + header `⋯ → Settings…` open it.
  The old `open-settings` event is no longer emitted.
- **Theme override**: setting `theme` = system|light|dark. `lib/theme.ts` sets `data-theme` on `<html>`; `tokens.css` has dark tokens under
  `@media (prefers-color-scheme: dark) :root:not([data-theme=light])` and `:root[data-theme=dark]`. Acrylic itself still follows Windows.
- **Default snooze** (`default_snooze`, default `10m`) preselects the toast dropdown (`defaultInput`). Settings defaults now also contain `theme`.
- **Hotkey recorder**: `lib/hotkey.ts` (vitest) builds the accelerator; `set_quickadd_hotkey` errors show inline. A *startup* registration
  failure is kept in `quickadd::STARTUP_ERROR` and surfaced via `quickadd_hotkey_error` so Settings shows it too. Known quirk: pressing the
  currently registered combo while recording triggers quick-add instead.
- **Updater**: `tauri-plugin-updater` + `tauri-plugin-process`, key at `%USERPROFILE%\.tauri\styky.key` (generated with an empty password;
  outside the repo, `*.key` is also git-ignored), public key in `tauri.conf.json`, endpoint placeholder `https://github.com/OWNER/REPO/...`.
  A failed check shows a friendly message. `scripts/build.ps1` loads the key into `TAURI_SIGNING_PRIVATE_KEY` and runs `pnpm tauri build`.
- **Icon**: `assets/icon.svg` (sticky note, folded corner) -> `pnpm tauri icon assets/icon.svg`; the tray uses the same app icon. The generated
  `android/` and `ios/` icon folders were deleted. Bundle `targets` is `["nsis"]` only.
- **Motion**: the springs, completion choreography (check, 250 ms, collapse), `:focus-visible` rings, press scale and empty state already existed from
  M1-M3; M4 added: reduced-motion fades for the motion springs/variants (`lib/motion.ts` reads `prefers-reduced-motion` at load), a global 0.97 press on
  menu items / toast actions, an empty state that shows the configured hotkey, and a spring-animated segmented control and switch in Settings.

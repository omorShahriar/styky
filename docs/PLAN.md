# Styky — Plan (v1)

A Windows 11 desktop widget that keeps crucial work reminders ("release `release/2.3`", "rotate API keys") permanently in front of you, with an Apple-grade feel.

Stack: **Tauri 2 (Rust) + React 18 + TypeScript + Vite**, pnpm. Deliverable: **per-user NSIS `Styky_x.y.z_x64-setup.exe`**.

---

## 1. Product decisions (from interview)

| Area | Decision |
|---|---|
| Form factor | One frosted-glass panel, resizable, draggable, remembers size/position/monitor |
| Z-order | Sits **behind all windows** (desktop layer) |
| Alerts | Native **Windows toast** with **Done** and **Snooze** buttons |
| Item fields | Title, markdown details, due date/time, **tag/project**, **priority** (low/normal/urgent), **repeat** (daily/weekdays/weekly/monthly), **checklists** (`- [ ]` in markdown, tickable), **link URL**, **branch/repo chip** (click copies) |
| Capture | **Global hotkey** (default `Ctrl+Alt+N`) opens a Spotlight-style quick-add bar; **natural language** ("Release v2.3 tomorrow 5pm #api !"); **+** button in panel opens full editor |
| Editing | Click an item and it **expands inline** with a spring animation showing rendered markdown; an **Edit** toggle switches to raw text |
| Completion | Done items move to a collapsible **Completed** section, can be restored, and auto-clear after N days (setting, default 7) |
| Look | Follows the Windows light/dark theme, Mica/Acrylic glass, Windows accent color |
| Storage | Local SQLite in `%APPDATA%\com.styky.app\`, plus JSON export/import |
| System | Launch at login, tray icon, close = hide, single instance, auto-update (code and key now, endpoint placeholder) |
| Snooze | 10 min · 1 hour · This evening (18:00) · Tomorrow morning (09:00) · Custom… |

**Out of scope for v1:** sync, multiple windows/notes, sounds, cloud accounts, mobile.

---

## 2. Architecture

```
styky/
  src/                      React UI
    app/                    windows: Panel, QuickAdd, Settings (routes by window label)
    components/             ItemRow, ItemDetail, MarkdownView, Editor, Chip, SnoozeMenu, ...
    lib/                    api.ts (typed invoke wrappers), nlp.ts (quick-add parser), time.ts
    styles/                 tokens.css (design tokens), globals.css
  src-tauri/
    src/
      main.rs / lib.rs      builder, plugins, tray, setup
      db.rs                 rusqlite (bundled), migrations, CRUD
      commands.rs           #[tauri::command] API used by UI
      scheduler.rs          due-time engine (tokio task)
      notify.rs             WinRT toast w/ actions (tauri-winrt-notification)
      window.rs             vibrancy, always-on-bottom, positioning
    tauri.conf.json
  docs/PLAN.md
```

**Rust owns data and time.** The UI never computes "is it due"; it renders state and receives `items-changed` events.

### Key crates and plugins
- `tauri` 2.x with `tray-icon`
- `tauri-plugin-window-state`, `tauri-plugin-autostart`, `tauri-plugin-single-instance`, `tauri-plugin-global-shortcut`, `tauri-plugin-updater`, `tauri-plugin-process`, `tauri-plugin-opener` (opening links), `tauri-plugin-clipboard-manager`, `tauri-plugin-dialog` (export/import)
- `window-vibrancy` (Mica on Win11, Acrylic fallback)
- `rusqlite` (feature `bundled`), `serde`, `chrono`, `uuid`, `tokio`
- `tauri-winrt-notification`. **Note:** the official notification plugin's action buttons are mobile-only, so toasts with Done/Snooze buttons must use WinRT directly. The AUMID must match the installed app; in `tauri dev` fall back to the PowerShell AUMID and document it.

### Frontend libs
- `motion` (Framer Motion) for springs and layout animations
- `react-markdown` + `remark-gfm` (task lists, tables) + `rehype-sanitize`; custom checkbox renderer that toggles the source line
- `chrono-node` for natural-language dates (custom tokens: `#tag`, `!`/`!!` priority, `every day|weekday|week|month`, `@branch`)
- `@fontsource-variable/inter` (bundled; SF-like) with `Segoe UI Variable` fallback
- Icons: `lucide-react`
- State: `zustand`. No UI kit; components are hand-built against the design tokens below.

---

## 3. Data model (SQLite, migration v1)

```sql
CREATE TABLE items (
  id TEXT PRIMARY KEY,               -- uuid v4
  title TEXT NOT NULL,
  body_md TEXT NOT NULL DEFAULT '',
  due_at INTEGER,                    -- unix ms UTC, nullable
  repeat TEXT NOT NULL DEFAULT 'none', -- none|daily|weekdays|weekly|monthly
  priority INTEGER NOT NULL DEFAULT 1, -- 0 low, 1 normal, 2 urgent
  tag TEXT,                          -- free text, color derived from hash
  link TEXT,
  branch TEXT,
  status TEXT NOT NULL DEFAULT 'open', -- open|done
  snoozed_until INTEGER,
  last_fired_at INTEGER,
  completed_at INTEGER,
  sort_order REAL NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
PRAGMA user_version = 1;
```

**Rules**
- Completing a repeating item does not mark it done. It advances `due_at` to the next occurrence (local time; monthly clamps to the end of the month; weekdays skips Sat/Sun) and clears `snoozed_until`/`last_fired_at`.
- Effective fire time is `snoozed_until ?? due_at`.
- **Sort:** open items by overdue first, then urgent, then due soonest, then no-due by `sort_order`. Users can drag-reorder no-due items.

**Commands:** `list_items`, `create_item`, `update_item`, `delete_item`, `complete_item`, `restore_item`, `snooze_item(id, until)`, `reorder`, `get_settings`, `set_setting`, `export_json(path)`, `import_json(path, mode: merge|replace)`.

---

## 4. Behavior specs

### Panel window (`main`)
- Frameless, transparent, `skipTaskbar: true`, default 360×560, min 300×320, top-right of the primary monitor on first run.
- **Mica** via `window-vibrancy` (Acrylic fallback), with an 8–12px rounded clip from Win11 DWM rounded corners.
- **Always-on-bottom** (`set_always_on_bottom(true)`); when focused it behaves normally.
- **Win+D / Show Desktop:** a normal bottom window gets hidden. Time-box 2h to investigate keeping it visible (e.g. re-show on `WM_SIZE` minimize, or WorkerW reparenting **only if** input still works). If no robust approach exists, accept the limitation and document it in README. Do not ship anything that breaks clicking or typing.
- Custom drag region in the header. Header shows "Styky", count of due today, `+`, and a `⋯` menu (Settings, Export, Import, Quit).
- Close button hides; Quit only from the tray or the menu.

### Item row
- Leading circular check (spring-scale on press, checkmark draws in), title, a meta line (`tag chip · relative due · repeat icon`), and an urgent dot.
- Overdue: due text and a subtle left accent use system red. Due within the next hour uses orange.
- Hover reveals quick actions: snooze, open link, copy branch.
- Click expands inline (motion `layout`), showing the rendered markdown, link, branch chip (monospace; click copies and shows a "Copied" micro-toast) and checklists that can be toggled. Esc or a click outside collapses it.
- **Edit mode:** title input, markdown textarea (monospace, auto-grow, `Ctrl+Enter` saves, `Esc` cancels), due date/time picker, repeat, priority segmented control, tag combo (existing tags suggested), link, branch. Changes autosave with a 400ms debounce.

### Quick-add window (`quickadd`)
- Global `Ctrl+Alt+N` (configurable) shows a centered 560×64 floating bar, always on top, Acrylic, autofocused.
- Live parse preview chips under the input (date, #tag, priority, repeat). Enter creates and hides; Esc hides; blur hides.
- Grammar examples: `Release v2.3 tomorrow 5pm #api !!`, `Rotate keys fri 10am #infra every month`, `Merge @release/2.3 in 2h`.

### Scheduler
- A Tokio task wakes at the next effective fire time, capped at 30s, and also re-plans whenever items change.
- On fire: show the toast (title, due time, tag) with actions **Done** and **Snooze ▾** (selection input with the presets; Custom opens the app at that item). Clicking the toast body focuses the panel and expands the item. Set `last_fired_at` so it never double-fires.
- **On startup:** items overdue while the app was off produce **one** summary toast ("3 reminders were due") rather than a burst.
- Handle sleep/resume and clock changes: compare wall clock on every tick and do not rely on sleep durations.

### Settings window
Theme (System/Light/Dark), quick-add hotkey recorder, launch at login, auto-clear completed after N days, default snooze, export/import, and an "About + check for updates" section.

### Tray
Left click toggles the panel. The menu has Show/Hide, Quick add, Settings, Pause reminders 1h, and Quit.

---

## 5. Design system ("feels like Apple, native on Windows 11")

- **Type:** Inter Variable. Title 15/20 semibold (−0.01em), body 13/18, meta 12/16 secondary. `font-feature-settings: "tnum", "cv11"` on times.
- **Color:** CSS variables per theme, plus the Windows accent read via Rust (`UISettings::GetColorValue(Accent)`) and exposed as `--accent`. Label colors follow Apple's primary, secondary and tertiary opacities (0.85 / 0.55 / 0.3). Panel tint is `rgba(255,255,255,0.55)` in light and `rgba(30,30,32,0.5)` in dark over Mica. Hairline separators are 0.5px at 8% opacity.
- **Spacing:** 4pt grid. Rows have 10px vertical padding and 14px horizontal padding; the panel has 12px padding.
- **Radius:** panel 12, cards 10, chips 6, buttons 8.
- **Motion:** springs only (`stiffness 400, damping 32` for press; `300/30` for expand), with no linear easing. Completing an item does check, then 250ms later the row collapses height and moves into Completed. List reorders use `layout`. Respect `prefers-reduced-motion` with fades only.
- **Feedback:** every press gives a 0.97 scale; focus rings use the accent at 2px for keyboard use only (`:focus-visible`).
- **Keyboard:** `↑/↓` move through items, `Enter` expands, `E` edits, `Space` completes, `S` snoozes, `Delete` deletes (with an undo toast for 5s), `Ctrl+N` opens the full editor.
- **Empty state:** a friendly single line ("Nothing pressing. Ctrl+Alt+N to add.") with a soft icon.
- Use the `apple-design` and `emil-design-eng` skills when building UI.

---

## 6. Milestones (each ends with a review gate)

**M0 — Toolchain & scaffold**
- Install via winget: `Rustlang.Rustup` (stable-x86_64-pc-windows-msvc) and `Microsoft.VisualStudio.2022.BuildTools` with `--override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`.
- `git init`, `.gitignore`, scaffold with `create-tauri-app` (React + TS + pnpm), then add the plugins.
- ✅ `pnpm tauri dev` opens a window.

**M1 — Shell**
- Frameless Mica panel, always-on-bottom, drag, window-state, tray, close-to-hide, single-instance, autostart toggle, theme and accent tokens.
- Do the Win+D investigation here.
- ✅ The panel survives restart in the same spot, toggles from the tray, and follows dark/light live.

**M2 — Core list**
- DB, migrations, commands, list UI, inline expand, markdown view with checkbox toggling, editor, tags, priority, link, branch chip, Completed section and auto-clear, export/import, keyboard nav.
- ✅ CRUD round-trips across restarts.

**M3 — Reminders & capture**
- Scheduler, repeat logic, WinRT toasts with Done/Snooze, snooze presets, overdue states, missed-on-startup summary, global hotkey quick-add, NL parser.
- ✅ Rust unit tests for next-occurrence, sort and effective time, and vitest tests for the NL parser.

**M4 — Polish & ship**
- Motion pass, empty states, settings window, app icon (simple rounded-square sticky glyph), updater (`tauri signer generate`, with the **private key stored outside the repo** at `%USERPROFILE%\.tauri\styky.key` and never printed or committed; endpoint placeholder `https://github.com/OWNER/REPO/releases/latest/download/latest.json`; `createUpdaterArtifacts: true`).
- `pnpm tauri build` produces NSIS `installMode: currentUser`.
- README with install, hotkeys and the Win+D note.
- ✅ The setup.exe installs, launches at login, and a toast fires from the installed build.

## 7. Acceptance checklist (final review)
- [ ] Installer runs without admin, creates a Start menu entry, uninstalls cleanly
- [ ] Panel is visible on the desktop at login, behind other windows, with glass that is correct in light and dark
- [ ] Quick-add via hotkey from any app parses date, tag, priority and repeat
- [ ] Toast fires on time with working Done and Snooze; nothing double-fires; missed items give one summary
- [ ] Repeating items roll forward correctly (including the monthly 31st edge case and weekdays)
- [ ] Markdown renders (GFM, code, links open in the browser, checklists toggle and persist)
- [ ] Export, then import (replace), is lossless
- [ ] `cargo clippy` is clean, `pnpm tsc --noEmit` is clean, and the tests pass
- [ ] No private key in the repo

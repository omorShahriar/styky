# Styky

A Windows 11 desktop widget that keeps the work reminders that matter ("release `release/2.3`", "rotate API keys")
permanently in front of you. One frosted-glass panel that sits on the desktop behind your windows, native Windows toasts
with **Done** and **Snooze** buttons, and a global hotkey for capturing a reminder in two seconds.

Tauri 2 (Rust) + React 19 + TypeScript. Local only: no account, no cloud, no sync.

## Install

1. Run `Styky_<version>_x64-setup.exe`. It installs **per user** (no administrator prompt) and adds a Start menu entry.
2. Launch **Styky** from the Start menu. The panel appears in the top-right of your primary monitor; a tray icon is added.
3. Turn on **Launch at login** in Settings (tray icon → Settings…) if you want it every day.

Uninstall from *Settings → Apps → Installed apps → Styky*. Your data is kept (see [Data location](#data-location)).

## Using it

- **Panel**: click a reminder to expand it (rendered markdown, checklists, link, branch chip). `Edit` switches to the editor.
  Close hides the panel; quit from the tray menu or `⋯ → Quit Styky`.
- **Tray icon**: left-click shows/hides the panel. Menu: Show / Hide, Quick add, Pause reminders 1h, Settings…, Launch at login, Quit.
- **Toasts**: when a reminder is due you get a Windows notification with a *Snooze* dropdown (10 min, 1 hour, this evening 18:00,
  tomorrow 09:00, Custom…) plus **Snooze** and **Done** buttons. Clicking the body opens the panel at that item.
  Reminders missed while Styky was not running produce one summary toast.
- **Repeats**: completing a repeating reminder rolls it to the next occurrence (monthly stays on its day, clamped to month end;
  weekdays skip the weekend).

### Hotkeys

| Keys | Action |
|---|---|
| `Ctrl+Alt+N` (global, configurable in Settings) | Open the quick-add bar from any app |
| `Enter` / `Esc` in the bar | Create the reminder / dismiss |

### Keyboard shortcuts (panel focused)

| Keys | Action |
|---|---|
| `↑` / `↓` | Move through reminders |
| `Enter` | Expand / collapse |
| `E` | Edit |
| `Space` | Complete (or restore a completed one) |
| `S` | Snooze menu |
| `Delete` | Delete (5 s undo toast) |
| `Ctrl+N` | New reminder in the full editor |
| `Ctrl+Enter` / `Esc` in the editor | Save and close / cancel |
| `Esc` | Collapse the expanded reminder |

### Quick-add grammar

Type a sentence; tokens can appear anywhere and the rest becomes the title. Chips under the field show what was understood.

| Token | Meaning | Example |
|---|---|---|
| any date/time | due time (anything [chrono-node](https://github.com/wanasit/chrono) understands) | `tomorrow 5pm`, `fri 10am`, `in 2h` |
| date without a time | due at 09:00 | `next monday` |
| `#tag` | tag / project | `#api` |
| `!!` | urgent | `Rotate keys !!` |
| `!` / `!low` | normal (explicit) / low priority | `!low` |
| `every day` · `every weekday` · `every week` · `every month` | repeat (a repeat with no date starts at the next 09:00) | `every month` |
| `@branch` | branch / repo chip (click to copy) | `@release/2.3` |
| a URL | link | `https://example.com/pr/12` |

Examples: `Release v2.3 tomorrow 5pm #api !!`, `Rotate keys fri 10am #infra every month`, `Merge @release/2.3 in 2h`.

## Settings

Theme (System / Light / Dark), quick-add hotkey recorder (shows an inline error if the combination is taken), launch at login,
default snooze (preselected in toasts), auto-clear completed after N days, JSON export / import (merge or replace), and
About with **Check for updates**.

## Data location

- Database: `%APPDATA%\com.styky.app\styky.db` (SQLite). Window position: `.window-state.json` in the same folder.
- Export from Settings → Data for a plain JSON backup.

## Known limitations

- **Win+D / Show Desktop.** Windows makes the desktop itself topmost while it is shown, which would hide a panel that sits behind
  your windows. Styky polls every 200 ms and raises the panel while no app window is visible, so after Win+D it can take up to
  ~0.2 s to reappear. Other "all windows minimised" states behave the same way. A full-screen exclusive app/game hides it.
- **Old toasts do nothing after Styky quits.** Done / Snooze / body-click are handled in-process. A toast left in the Action
  Center while Styky is not running will not react when clicked. Reminders are re-evaluated when Styky starts.
- **Pause reminders** is held in memory only and resets on restart.
- **Updater endpoint is a placeholder** (see below), so *Check for updates* shows a friendly "couldn't check" message until you wire it up.
- **Dev builds** show toasts under "Windows PowerShell" (the installed app uses the Styky identity).
- Quick-add hotkey recording: pressing the *currently registered* combination while recording triggers the quick-add bar
  instead of recording it; pick any other combination.

## Auto-update (wire it to GitHub Releases)

`tauri-plugin-updater` is built in and the update public key is already in `src-tauri/tauri.conf.json`. Placeholder endpoint:

```
https://github.com/OWNER/REPO/releases/latest/download/latest.json
```

1. Replace `OWNER/REPO` in `plugins.updater.endpoints` (`src-tauri/tauri.conf.json`) with your repository.
2. Bump `version` in `tauri.conf.json`, `package.json` and `src-tauri/Cargo.toml`, then run `scripts\build.ps1`. It reads the signing key
   from `%USERPROFILE%\.tauri\styky.key` (never stored in the repo) and builds the NSIS installer plus its `.sig` signature
   (`src-tauri\target\release\bundle\nsis\`).
3. Create a GitHub release for the new tag and upload the `Styky_<version>_x64-setup.exe` (and its `.exe.sig`), plus a `latest.json`:

   ```json
   {
     "version": "0.2.0",
     "notes": "What changed",
     "pub_date": "2026-01-01T00:00:00Z",
     "platforms": {
       "windows-x86_64": {
         "signature": "<contents of Styky_0.2.0_x64-setup.exe.sig>",
         "url": "https://github.com/OWNER/REPO/releases/download/v0.2.0/Styky_0.2.0_x64-setup.exe"
       }
     }
   }
   ```

   `releases/latest/download/latest.json` always resolves to the newest release, so installed copies find it automatically.

Keep `styky.key` safe and backed up: an app signed with a new key can no longer be updated by installs that trust the old public key.
To make a new key: `pnpm tauri signer generate -w %USERPROFILE%\.tauri\styky.key`, then put the new `.pub` contents in `tauri.conf.json`.

## Develop

Requirements: Windows 11, Node + pnpm, Rust (`stable-x86_64-pc-windows-msvc`) and the VS 2022 Build Tools C++ workload.

```powershell
pnpm install
pnpm tauri dev         # run
pnpm test              # vitest
cd src-tauri; cargo test; cargo clippy --all-targets
pnpm tsc --noEmit
.\scripts\build.ps1    # signed installer
```

More detail (module map, API, gotchas, deviations from the plan): [`docs/NOTES.md`](docs/NOTES.md); the spec is [`docs/PLAN.md`](docs/PLAN.md).

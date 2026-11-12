<div align="center">

<img src="assets/icon.svg" width="96" alt="Styky icon" />

# Styky

**Remember the small stuff that breaks big things.**

A small desktop reminder widget for Windows 11. It stays visible after Win+D and nudges you with notifications you can act on.
Get a native notification when it is due, with Done and Snooze on it. The always-visible glass panel on your desktop shows what is coming. Local-first: no account, no cloud,
no telemetry.

<!-- [[MCP BLOCK]] -->
<!--
```powershell
styky add "Deploy api fri 5pm #infra !!"      # free
claude mcp add styky -- styky mcp             # free: all seven tools
```
-->
<!-- [[/MCP BLOCK]] -->

[![Latest release](https://img.shields.io/github/v/release/omorShahriar/styky?label=download&color=7c5cff)](https://github.com/omorShahriar/styky/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Windows 11](https://img.shields.io/badge/Windows-11-0078d4?logo=windows11&logoColor=white)
![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24c8db?logo=tauri&logoColor=white)

[**Download for Windows**](https://github.com/omorShahriar/styky/releases/latest) · [Website](https://styky.smallhingelabs.com/readme) <!-- [[MCP BLOCK]] --><!-- · [Claude, ChatGPT & terminal](#use-it-from-claude-chatgpt-and-the-terminal)--><!-- [[/MCP BLOCK]] --> · [Quick add](#quick-add-type-it-like-you-say-it) · [Styky Pro](#styky-pro) · [Build from source](#build-from-source)

</div>

---

## Why Styky?

Task managers are great for projects, but the most dangerous tasks are the small, single ones that live only in your head
until it's too late: *deploy on Friday*, *rotate the keys*, *check CI after the push*. Styky is built around three ideas:

- **Set it from anywhere.** `Ctrl+Alt+N` from any app.
<!-- [[MCP BLOCK]] -->
<!--
- **Set it from anywhere.** `Ctrl+Alt+N` from any app, `styky add` from a terminal or git hook, or ask Claude through MCP.
  They all share one grammar: `Deploy api fri 5pm #infra !!`.
-->
<!-- [[/MCP BLOCK]] -->
- **It tells you on time.** A native Windows notification fires when the reminder is due, with **Done** and **Snooze**
  on it. Every priority, Critical (`!!!`) included, is free. With Pro you can switch on **Nag** for a reminder, on any
  priority, and it re-notifies every 10 minutes (configurable) until you complete or snooze it.
- **Visible and local.** One frosted-glass panel lives on the desktop layer behind your windows. Reminders sit in a SQLite
  file on your machine, and the local API is a named pipe only your Windows user can open.

## Features

- 🪟 **Desktop widget.** A resizable Acrylic glass panel that sits behind your windows, follows light/dark mode and
  your Windows accent color, and remembers its size and position.
- ⚡ **Global quick-add.** A Spotlight-style bar with natural-language dates, `#tags`, `!!` priority, repeats and links.
- 🔔 **Native notifications.** Windows toasts with a Snooze dropdown (10 min, 1 hour, this evening, tomorrow, custom) and
  Done. Missed reminders are summarized in a single toast, not a flood.
- 📝 **Markdown details.** Notes, code, links, and **checklists you can tick** right in the widget.
- 🔁 **Repeats.** Daily, weekdays, weekly, monthly (the 31st stays the 31st and clamps to shorter months).
- 🧑‍💻 **Developer-friendly.** A branch chip (`@release/2.3`) that copies on click, a link field, and project tags.
- ⌨️ **Keyboard-first.** Navigate, complete, snooze, edit and delete without touching the mouse.
- ✨ **Feels native.** Spring animations, reduced-motion support, light and dark themes.
- 🔒 **Private.** Everything stays local, with JSON export/import for backups.
- 🚀 **Starts with Windows.** One click in Settings installs signed updates from GitHub Releases.

## Install

**Download:** grab `Styky_<version>_x64-setup.exe` from the [latest release](https://github.com/omorShahriar/styky/releases/latest)
and run it.

- It installs **per user**, with no admin prompt, and adds a Start menu entry.
<!-- [[MCP BLOCK]] -->
<!--
- From v0.2.0 the installer also puts the `styky` command on your user `PATH` (open a new terminal afterwards).
-->
<!-- [[/MCP BLOCK]] -->
- Styky starts at login by default. You can change that in Settings.
- Requires Windows 10 1809+ / Windows 11 (the WebView2 runtime is already part of Windows 11).
- **Microsoft Store:** a listing is in review.
<!-- [[MCP BLOCK]] -->
<!--
- **Microsoft Store:** a listing is in review. The Store build does not include the `styky` CLI or MCP server yet; use the GitHub installer for those.
-->
<!-- [[/MCP BLOCK]] -->

> Windows SmartScreen may warn about an unrecognized app because the installer isn't Authenticode-signed yet. Click
> **More info → Run anyway**. Updates are cryptographically signed and verified by the app itself.

Uninstall from *Settings → Apps → Installed apps → Styky*. Your data is kept (see [Your data](#your-data)).

<!-- [[MCP BLOCK]] -->
<!--
## Use it from Claude, ChatGPT and the terminal

*New in v0.2.0.* Styky ships a `styky` command and a local MCP server. Both talk to the **running** Styky app over a
named pipe (`\\.\pipe\styky-<username>`) whose access is limited to your Windows account; nothing listens on the network.
If Styky is not running you get `Styky isn't running` and exit code 2.

### Install

Install Styky from the [GitHub installer](https://github.com/omorShahriar/styky/releases/latest) (v0.2.0 or newer). It
adds `<install dir>\bin\styky.exe` to your user `PATH`. Check with:

```powershell
styky --version
```

### CLI

| Command | What it does |
|---|---|
| `styky add <text...> [--json]` | Add a reminder from a natural-language line (same grammar as `Ctrl+Alt+N`) |
| `styky ls [--today \| --overdue] [--tag <t>] [--done \| --all] [--json]` | List reminders (open by default) |
| `styky get <id> [--json]` | Show one reminder with its notes |
| `styky done <id>` | Complete it (a repeating reminder rolls to its next time) |
| `styky snooze <id> <when>` | e.g. `1h`, `30m`, `"tomorrow 9am"` |
| `styky edit <id> [--title t] [--due when\|none] [--tag t] [--priority low\|normal\|urgent\|critical] [--notes n] [--repeat r] [--link u] [--branch b]` | Change fields |
| `styky rm <id>` | Delete it |
| `styky mcp` | Run the MCP server on stdio |

Every command is free. `<id>` is the 8-character id shown by `styky ls`; any unique prefix works. A request that
uses a Pro feature (an advanced repeat such as `FREQ=WEEKLY;INTERVAL=2`) is refused on its own: the CLI prints
`Advanced repeats is a Styky Pro feature: https://styky.smallhingelabs.com/pro` and exits with code 3.

```powershell
styky add "Deploy api fri 5pm #infra !!"
styky ls --today
styky ls --overdue --tag infra --json
styky add "Rotate prod keys fri 10am #infra !!!"    # !!! = Critical
styky done a1b2c3d4
```

### Git hook

Git has no client-side `post-push` hook, so use `pre-push`. Save this as `.git/hooks/pre-push` (Git for Windows runs it
as is). It queues a CI check for the branch as you push; if the push is rejected, the reminder is still there to dismiss.

```sh
#!/bin/sh
branch=$(git rev-parse --abbrev-ref HEAD)
styky add "Check CI in 15m #ci @$branch" || true
exit 0
```

### MCP server

`styky mcp` is a stdio MCP server (protocol versions 2025-11-25, 2025-06-18, 2025-03-26, 2024-11-05 are accepted). It
exposes seven tools:

| Tool | Purpose |
|---|---|
| `styky_add` | Add a reminder from `text` (natural language) or explicit fields: `title`, `due`, `tag`, `priority` (`low`/`normal`/`urgent`/`critical`), `repeat`, `notes`, `link`, `branch` |
| `styky_list` | List reminders; filter by `due` (`today`/`overdue`/`any`), `tag`, `status` (`open`/`done`/`all`) |
| `styky_get` | One reminder with all fields, including notes |
| `styky_complete` | Mark done (repeating reminders roll forward) |
| `styky_snooze` | Notify again at a given time |
| `styky_update` | Change fields; `due: null` clears the due time |
| `styky_delete` | Permanently delete |

All seven tools are free. A request that uses a Pro feature (an advanced repeat) comes back as a tool error with the
code `requires_pro`, the feature name and the link, plus a plain sentence the assistant can relay. The rest of the
tool keeps working.

**Claude Code**

```powershell
claude mcp add styky -- styky mcp
```

**Claude Desktop**: add to `claude_desktop_config.json` (Settings, Developer, Edit Config) and restart Claude:

```json
{
  "mcpServers": {
    "styky": {
      "command": "styky",
      "args": ["mcp"]
    }
  }
}
```

**Other MCP clients** (Cursor, VS Code, Codex and anything that can launch a local stdio server): command `styky`,
argument `mcp`. For example:

```json
{ "styky": { "type": "stdio", "command": "styky", "args": ["mcp"] } }
```

Settings → **Connect AI** in the app shows the same snippets with a copy button.

**ChatGPT** only connects to remote HTTPS MCP servers today, so it cannot launch a local one such as `styky mcp`. Use the CLI
from scripts or any client that can run a local command.

> Privacy note: what an assistant reads through these tools (titles, due times, notes) goes to that assistant. Connect only
> clients you trust. Styky itself sends nothing anywhere.
-->
<!-- [[/MCP BLOCK]] -->

## Using Styky

| Where | What you can do |
|---|---|
| **Panel** | Click a reminder to expand it (markdown, checklists, link, branch chip). **Edit** opens the editor. Closing the panel hides it. |
| **Tray icon** | Left-click shows or hides the panel. The menu has Quick add, Pause reminders for 1 hour, Settings, Launch at login, and Quit. |
| **Notifications** | **Done** completes the reminder. Pick a time and click **Snooze**. Click the body to jump to the reminder. |
| **Settings** | Theme, quick-add hotkey, launch at login, default snooze, auto-clear for completed reminders, export/import, and update checks. |

### Quick add: type it like you say it

Press `Ctrl+Alt+N` (configurable) from any app and type a sentence. Tokens can go anywhere, and whatever is left becomes the
title. Live chips under the field show what Styky understood.

| You type | Styky understands |
|---|---|
| `tomorrow 5pm`, `fri 10am`, `in 2h`, `next monday` | due time (a date with no time is due at 09:00) |
| `#api` | tag / project |
| `!!` · `!` · `!low` | urgent · normal · low priority |
| `every day` · `every weekday` · `every week` · `every month` | repeat |
| `@release/2.3` | branch chip (click to copy) |
| `https://…` | link |

```text
Release v2.3 tomorrow 5pm #api !!
Rotate keys fri 10am #infra every month
Merge @release/2.3 in 2h
```

### Keyboard shortcuts

| Keys | Action |
|---|---|
| `Ctrl+Alt+N` (anywhere) | Quick add |
| `↑` `↓` | Move between reminders |
| `Enter` | Expand / collapse |
| `E` | Edit |
| `Space` | Complete (or restore) |
| `S` | Snooze |
| `Delete` | Delete (with 5-second undo) |
| `Ctrl+N` | New reminder in the full editor |
| `Ctrl+Enter` / `Esc` | Save / cancel in the editor |

## Your data

- Reminders are stored in `%APPDATA%\com.styky.app\styky.db` (SQLite) on your machine and nowhere else.
- Settings → Data → **Export** creates a plain JSON backup, and **Import** can merge or replace. Exports never contain your Pro license key.
- No telemetry. Styky only goes online to check for updates and to activate or remove a Pro key when you click the button.

## Styky Pro

Styky stays free and MIT-licensed, and nothing that was free becomes paid. **Styky Pro** is an optional one-time unlock
(**$12**, no subscription, coming Nov 30):

- **Nagging**: switch it on per reminder, any priority, and it re-notifies until you act
- early warnings: a heads-up before the due time (5 minutes to 1 day before)
- advanced repeats (every 2 weeks, Mon/Wed/Fri, the last Friday of the month, end after N times or on a date)
- themes (tint, transparency, density, accent, paper skin, font size)

It runs on the honor system: the code is in this repo behind a license check, and you can build it yourself. Buying supports
the project and gets you the signed, auto-updating build. Details: **[styky.smallhingelabs.com](https://styky.smallhingelabs.com/#pricing)**.

## Known limitations

- **Win+D:** while the desktop is shown, Styky raises the panel above it. This can take up to ~0.2 s.
- **Full-screen games** and exclusive full-screen apps cover the panel, like any other window.
- **Old notifications:** clicking a notification left in Action Center does nothing if Styky has been quit. Closing the panel only
  hides it, so this rarely matters.
- **Pause reminders** resets when Styky restarts.

## Build from source

You need Windows 10/11, [Node.js](https://nodejs.org) 20+ with [pnpm](https://pnpm.io), [Rust](https://rustup.rs)
(`stable-x86_64-pc-windows-msvc`), and the Visual Studio 2022 Build Tools with the **Desktop development with C++** workload.

```powershell
git clone https://github.com/omorShahriar/styky.git
cd styky
pnpm install
pnpm tauri dev          # run with hot reload
```

| Task | Command |
|---|---|
| Frontend tests | `pnpm test` |
| Rust tests + lint | `cd src-tauri; cargo test; cargo clippy --all-targets` |
| Type check | `pnpm tsc --noEmit` |
| Installer (unsigned updater artifacts) | `pnpm tauri build` |

In dev builds, notifications appear under "Windows PowerShell" because only the installed app has the Styky identity.

### Tech stack

[Tauri 2](https://tauri.app) (Rust) · React 19 · TypeScript · Vite · SQLite ([rusqlite](https://github.com/rusqlite/rusqlite)) ·
[Motion](https://motion.dev) · [chrono-node](https://github.com/wanasit/chrono) · WinRT toasts via the [`windows`](https://github.com/microsoft/windows-rs) crate.

### Project docs

- [`docs/PLAN.md`](docs/PLAN.md) is the product spec and design system.
- [`docs/NOTES.md`](docs/NOTES.md) covers architecture, the module map, the command API, and the Windows quirks we hit (z-order, Mica vs Acrylic, toast activation).
- [`docs/RELEASING.md`](docs/RELEASING.md) explains how releases and signed updates are produced.

## Contributing

Issues and pull requests are welcome. Before you open a PR:

1. Keep `pnpm test`, `pnpm tsc --noEmit`, `cargo test` and `cargo clippy --all-targets` clean.
2. Match the existing design language: springs, not linear easing, and tokens from `src/styles/tokens.css`.
3. For anything big, open an issue first so we can agree on the approach.

## License

[MIT](LICENSE) © 2026 Omor Shahriar

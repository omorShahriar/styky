<div align="center">

<img src="assets/icon.svg" width="96" alt="Styky icon" />

# Styky

**The reminders you can't afford to forget, always on your desktop.**

A small, glassy Windows 11 widget for those one-off but crucial things: *release `release/2.3` on Friday*, *rotate the API keys*,
*reply to the client before 5*. It sits on your desktop behind your windows, nudges you with a native notification when
something is due, and stays out of the way the rest of the time.

[![Latest release](https://img.shields.io/github/v/release/omorShahriar/styky?label=download&color=7c5cff)](https://github.com/omorShahriar/styky/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Windows 11](https://img.shields.io/badge/Windows-11-0078d4?logo=windows11&logoColor=white)
![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24c8db?logo=tauri&logoColor=white)

[**Download for Windows**](https://github.com/omorShahriar/styky/releases/latest) · [Features](#features) · [Quick add](#quick-add-type-it-like-you-say-it) · [Build from source](#build-from-source)

</div>

---

## Why Styky?

Task managers are great for projects, but the most dangerous tasks are the small, single ones that live only in your head
until it's too late. Styky is designed around three ideas:

- **Always visible.** One frosted-glass panel lives *on the desktop layer*, behind your apps, so you see it every time
  you glance at your desktop, even after Win+D.
- **Capture in two seconds.** Press `Ctrl+Alt+N` anywhere, type `Release v2.3 tomorrow 5pm #api !!`, hit Enter. Done.
- **Nudges, not nagging.** A native Windows notification fires on time, with **Done** and **Snooze** right on it.

It's local-first: no account, no cloud, no telemetry. Your reminders stay in a SQLite file on your machine.

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
- Styky starts at login by default. You can change that in Settings.
- Requires Windows 10 1809+ / Windows 11 (the WebView2 runtime is already part of Windows 11).
- **Microsoft Store:** coming soon.

> Windows SmartScreen may warn about an unrecognized app because the installer isn't Authenticode-signed yet. Click
> **More info → Run anyway**. Updates are cryptographically signed and verified by the app itself.

Uninstall from *Settings → Apps → Installed apps → Styky*. Your data is kept (see [Your data](#your-data)).

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
- Settings → Data → **Export** creates a plain JSON backup, and **Import** can merge or replace.
- Styky only touches the network when you click **Check for updates**, which queries this repository's releases.

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

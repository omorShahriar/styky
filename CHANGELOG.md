# Changelog

## 0.2.1

Fixes the reminders list staying empty for up to a minute after Windows starts.

### Fixed

- After a cold start the widget could appear before its data was ready, so the list stayed empty until the next refresh. The
  data is now ready before any window loads, and an early failed load retries within a fraction of a second.
- Launch at login no longer blocks the app while it is checked on the Store package.

## 0.2.0

Adds a command line and an MCP server so scripts and AI agents can add reminders that appear on your desktop.

### Added

- **`styky` command line.** `styky add`, `ls`, `get`, `done`, `snooze`, `edit` and `rm` talk to the running app. `add` uses the same
  grammar as `Ctrl+Alt+N` (`styky add "Deploy api fri 5pm #infra !!"`). The installer puts `styky` on your PATH. If Styky is not running
  the command says so and exits with code 2.
- **MCP server.** `styky mcp` is a stdio Model Context Protocol server with seven tools: `styky_add`, `styky_list`, `styky_get`,
  `styky_complete`, `styky_snooze`, `styky_update` and `styky_delete`. Settings → Connect AI shows copy-ready setup for Claude Code,
  Claude Desktop, Cursor and VS Code.
- **Local API.** The CLI and the MCP server reach the app over a Windows named pipe that only your Windows user can open. The app
  stays the only writer of the database. No port is opened.
- **Critical priority** (`!!!` in quick add, or Critical in the editor), alongside Low, Normal and Urgent.
- **About** in Settings now states what Styky sends over the network.

### Changed

- Notifications are shown off the scheduler loop with a timeout, so one stuck toast cannot delay the next reminder.
- The editor shows all four priorities.
- The release build now publishes `Styky_x64-setup.exe`, a fixed-name copy of the installer, next to the versioned one.

### Notes

- The Microsoft Store package does not include the command line or the MCP server yet. Use the GitHub installer for those.
- Styky still sends nothing except update checks. See [docs/PRIVACY.md](docs/PRIVACY.md).

## 0.1.0

First public release: the desktop reminder panel, `Ctrl+Alt+N` quick add with natural-language dates, native toasts with Done and
Snooze, tags, links, branch chips, Markdown notes, repeats, JSON export and import, and signed automatic updates.

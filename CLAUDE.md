# Styky: engineering conventions

Styky is a Windows desktop reminder widget (Tauri 2, React 19, SQLite). Local-first, MIT.

If `docs/PRODUCT_CONTEXT.md` exists in your checkout, read it before any product-facing change (UI strings, README, website,
Free/Pro gating, MCP/CLI, licensing, network calls, Store packaging). Code is fact and that file is intent: if they disagree,
say so in your report instead of silently picking one.

## Where things are
- App: `src/` (React) and `src-tauri/src/` (Rust). The CLI is a second binary, `src-tauri/src/cli/`. The marketing site is `site/`
  (its own `package.json`; run `pnpm install --ignore-workspace` there).
- Notes, handoffs and known gotchas: `docs/NOTES.md`. Release steps and Store (MSIX) packaging: `docs/RELEASING.md`.
  Privacy policy: `docs/PRIVACY.md`.

## Checks to keep green
- `pnpm test`, `pnpm tsc --noEmit`, and for the site `pnpm typecheck` and `pnpm build` (in `site/`).
- In `src-tauri/`: `cargo test` and `cargo clippy -- -D warnings`.

## Conventions
- **Privacy:** no telemetry. Any new network call needs the owner's approval and a matching `docs/PRIVACY.md` update. The app is the
  only SQLite writer; the CLI and MCP server talk to it over the per-user named pipe.
- **Pro:** gated by `license::is_pro()`; an unshipped Pro feature never appears in the UI, a shipped one is labelled "Pro".
- **Copy:** calm, short, dev-native. No hype words.
- **Windows environment:** many files are CRLF, so normalise line endings in scripted edits. Use the editor tools instead of shell
  heredocs for files with backslashes or apostrophes. Never broad-kill `node` or `cargo`.
- **Dev instance next to an installed Styky:** run it with another identifier (`pnpm tauri dev --config` with
  `{"identifier":"com.styky.dev"}`), otherwise single-instance hands over to the installed app and both share a database.
- **Never install or run the NSIS or MSIX build from inside a Claude desktop app session.** The app is MSIX-packaged, so processes it
  spawns get file and registry virtualization and the result is not what a real user gets. Have the owner double-click the installer.
- **Z-order:** never `HWND_BOTTOM` / `set_always_on_bottom`; see `src-tauri/src/desktop.rs`. A new topmost window must call
  `desktop::suspend`.
- **Tauri permissions** are per window in `src-tauri/capabilities/`; a new window needs its own entry.

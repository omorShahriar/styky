# Styky Pro — Plan

**Model:** Styky stays free and MIT-licensed. **Styky Pro** is a **one-time $6.99** lifetime unlock that adds power features
on top. Nothing that is free today becomes paid.

## Principles

- **Free stays complete.** The panel, quick-add, natural-language parsing, toasts, snooze, repeats, markdown, tags, export,
  unlimited reminders, and *basic* AI/CLI capture are always free.
- **One license system everywhere.** A license key sold through a merchant of record (Polar.sh preferred because it is
  built for open source; Lemon Squeezy is the alternative) unlocks Pro in both the GitHub/NSIS build and the Microsoft Store build.
  Non-game Store apps may use third-party commerce and keep 100% of revenue. Tick the third-party purchase box in
  Partner Center (policy 10.8.2.x).
- **The Store app stays free.** Pro is an optional unlock, which keeps us clear of Store policy 10.8.7 (profiting from open
  source that is freely available). The original author selling an add-on unlock is fine.
- **Honor system, not DRM.** The Pro code lives in the public repo behind a license check. Anyone can build it themselves;
  people pay for convenience and to support the project. Revisit (open-core / private module) only if this clearly fails.
- **Offline-friendly.** Validate the key once online, then cache a signed entitlement locally. Revalidate occasionally with a long
  grace period, so Pro never breaks on a plane.

## Pro v1 (launch)

### 1. Local API, CLI and AI connectors (Claude / ChatGPT)
One local API in the app; every "connect" feature is a thin client of it.

- **Local API:** a named pipe (`\\.\pipe\styky`), or localhost HTTP bound to 127.0.0.1 with a per-install token stored in
  `%APPDATA%`. Operations: `add`, `list`, `get`, `complete`, `snooze`, `update`, `delete`. The NL parser is shared with quick-add
  (`add "Release v2.3 fri 5pm #api !!"`).
- **`styky` CLI:** a small Rust binary shipped with the app and added to the user PATH:
  `styky add "…"`, `styky ls --due today --json`, `styky done <id>`. It also makes **git hooks** possible, e.g. a post-push hook
  that reminds you to merge.
- **MCP server:** `styky mcp` (stdio), exposing tools to Claude Desktop, Claude Code, ChatGPT connectors and any MCP client, with
  one-click setup instructions in Settings. This is what lets someone say "remind me to rotate the keys Friday" to Claude.
- **Tiering:**
  - **Free:** `add`, `list`.
  - **Pro:** `complete`, `snooze`, `update`, `delete`, bulk operations, filters, plus AI quick-add (later).

  Basic free access gets Styky listed in MCP directories, which is a discovery channel.

### 2. Escalating reminders and pre-alerts
- Pre-alert per item: 5, 15 or 30 minutes, or 1 hour, or 1 day before.
- **Escalation** for urgent items: re-notify every N minutes until done, optionally bring the panel forward, and optionally play a sound.
- A "Critical" priority level that uses escalation by default.

### 3. Themes and advanced repeats
- **Themes:** panel tint and transparency slider, density (comfortable/compact), accent override, a "sticky note" paper skin,
  and font size.
- **Advanced repeats:** every N days/weeks/months, specific weekdays (Mon/Wed/Fri), the last or nth weekday of the month
  ("last Friday"), and end after N times or on a date. Natural-language support: `every 2 weeks`, `last friday of the month`.

### 4. Licensing plumbing
- Settings → **Styky Pro** card: the benefits, a **Buy ($6.99)** link to the checkout, a **key field**, and the activated state with a
  supporter badge ♥.
- Rust `license.rs`: validates against the provider's license API (activation limit about 3 devices), caches the entitlement,
  and gates features with `is_pro()`. Locked Pro controls show a small "Pro" pill that opens the card, with no nag dialogs.
- Keep the provider's API key out of the app; key validation uses public, per-license endpoints only.

## Roadmap (Pro v2+)
- **GitHub watch:** link a reminder to a repo, branch, PR or tag. It shows live status (open, merged, CI failing) and
  auto-completes or alerts on merge or a new tag. Uses a GitHub token (device flow).
- **Multiple boards/panels:** a panel per project or client, each with its own position, monitor or virtual desktop.
- **VS Code extension:** capture a reminder from a selection or comment, and show due items in the status bar (via the local API).
- **Calendar / week timeline** view.
- **Integrations import:** GitHub Issues, Jira and Linear assigned items as reminders.
- **AI quick-add:** paste a Slack or email message and get a structured reminder (bring your own Claude/OpenAI key, or via MCP).
- **Sync via a folder** (OneDrive/Dropbox) for multiple PCs.

## Pricing notes
- $6.99 one-time (launch price $4.99 for the first weeks). One license covers about 3 personal devices.
- Major future versions remain covered ("all future Pro features"). Revisit only if Styky ever needs servers.

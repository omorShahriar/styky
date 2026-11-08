# Styky Pro — Plan and v1 decisions

**Model:** Styky stays free and MIT-licensed. **Styky Pro** is a **one-time unlock** (early-bird **$4.99**, then **$6.99**)
that adds power features on top. Nothing that is free today becomes paid. Sold through **Lemon Squeezy** (merchant of record).

> Status: **Pro v1 is implemented** (branch `claude/pro-v1-*`; see `docs/NOTES.md` → "Pro v1 notes" for what was built, how it was
> verified and what is left for the owner). The sections below are the plan with the final decisions folded in.

## Principles

- **Free stays complete.** The panel, quick-add, natural-language parsing, toasts, snooze, simple repeats, markdown, tags, export,
  unlimited reminders, and the *basic* CLI/AI access (`add`, `list`) are always free.
- **One license system everywhere.** A Lemon Squeezy license key unlocks Pro in the GitHub/NSIS build and (when the Store build
  ships) the Microsoft Store build. The key is checked against Lemon Squeezy's public License API; no secret key is in the app.
  Why Lemon Squeezy: it is a merchant of record that pays out to Bangladesh (Stripe and Polar do not), handles VAT, and has
  license keys built in.
- **The Store app stays free.** Pro is an optional unlock, which keeps us clear of Store policy 10.8.7. (The Store is postponed;
  the MSIX code in `store.rs` keeps compiling. A Store build would also need an `appExecutionAlias` for the CLI.)
- **Honor system, not DRM.** The Pro code is in the public repo behind a license check. Anyone can build it themselves; people pay
  for convenience and to support the project. Revisit (open-core / private module) only if this clearly fails.
- **Offline-friendly.** Activate once online; the entitlement is cached locally. Revalidate weekly; Pro keeps working for **30 days**
  without a successful check, and only an *explicit* rejection from the API removes it.

## Licensing (decisions)

- `src-tauri/src/license.rs`, endpoints `POST https://api.lemonsqueezy.com/v1/licenses/{activate,validate,deactivate}` (form body,
  no auth). Activation `instance_name` = `<PC name> Styky`.
- Cached in the SQLite table `license` (`key`, `instance_id`, `validated_at`, `customer`), deliberately **not** in `settings`, so
  Export never carries the key.
- State machine (all unit-tested with a mocked HTTP layer): `validated_at` within 30 days ⇒ Pro; re-check when older than 7 days
  (at start and every 6 h); `valid:false` / `disabled` / `expired` / unknown key or instance ⇒ the entitlement is deleted;
  offline, timeout, 5xx, 429 or garbage ⇒ nothing changes. After 30 days without a successful check Pro pauses (state
  `grace-expired`, the key is kept) and comes back by itself once a validation succeeds.
- **Remove license** deactivates the instance (frees the slot) and forgets the key, even when offline.
- **Product check:** `LEMON_STORE_ID` / `LEMON_PRODUCT_ID` in `license.rs` (`0` = not configured yet ⇒ no check). Set both before
  selling, or keys from any Lemon Squeezy store would unlock Pro. `STYKY_BUY_URL` is the checkout link (placeholder).
- `license::is_pro()` gates features; the `pro-changed` event refreshes every window. `STYKY_PRO_DEV=1` forces Pro on in **debug**
  builds only.
- Settings → **Styky Pro** card: benefits, **Buy**, key field with activate states, activated state with a ♥ Supporter badge,
  Remove license. Locked controls show a small **Pro** pill that opens the card. No nag dialogs. Activation limit (about 3 PCs) is set
  on the Lemon Squeezy product.

## Pro v1 features

### 1. Local API, CLI and AI connectors
- **Local API:** named pipe `\\.\pipe\styky-<username>`, newline-delimited JSON (`{"id","method","params"}` →
  `{"id","result"|"error":{"code","message"}}`), created with an ACL that grants only the current user, remote clients rejected and
  `first_pipe_instance` so another process cannot squat the name. Methods `add`, `list`, `get`, `complete`, `snooze`, `update`,
  `delete`.
- **Tiering:** **Free:** `add`, `list` (including the due-today / overdue / tag / open-done filters). **Pro:** `get`, `complete`,
  `snooze`, `update`, `delete`; the app answers with the error code `requires_pro` and a plain message. (Deviation from the first
  draft, which listed "filters" as Pro: filters are part of `list`, which stays free.)
- **`add` takes natural language** and also structured fields (`title`, `due`, `tag`, `priority`, `repeat`, `notes`, `link`,
  `branch`). The NL parser is a **Rust port** (`nlp.rs`, `nlp_repeat.rs`) rather than a call into the webview: the API must work with
  the panel never shown, mid-start-up or throttled, and from a headless CLI. The quick-add bar keeps chrono-node.
- **`styky` CLI** (`styky add|ls|get|done|snooze|edit|rm|mcp`, `--json`). A second Rust binary (`styky-cli`), shipped by the
  installer as `<install>\bin\styky.exe`; the NSIS hook adds `bin` to the *user* PATH and removes it on uninstall (script
  `installer/styky-path.ps1`).
- **`styky mcp`**: a stdio MCP server exposing seven tools (`styky_add`, `styky_list`, `styky_get`, `styky_complete`,
  `styky_snooze`, `styky_update`, `styky_delete`) with descriptions and annotations. Hand-rolled instead of the `rmcp` SDK:
  five JSON-RPC methods do not justify tokio + schemars + proc-macros in the CLI.
- Settings → **Connect AI**: copy-paste config for Claude Desktop, Claude Code and other MCP clients (ChatGPT only supports remote
  HTTPS MCP servers today, so it cannot use a local stdio one directly).

### 2. Escalating reminders and pre-alerts
- Per-item pre-alert: 5, 15, 30 minutes, 1 hour or 1 day before the due time (a heads-up toast, once; skipped while snoozed).
- **Critical** priority (3): the toast re-fires every N minutes (setting, default 10) until done or snoozed, replacing the previous
  toast instead of stacking; optional "bring the panel forward". `!!!` in quick-add. Free builds fall back to Urgent.

### 3. Themes and advanced repeats
- **Themes:** tint, transparency slider, density (comfortable/compact), accent override, **Sticky** paper skin, font size; all
  applied through CSS tokens (`lib/proTheme.ts`, `styles/tokens.css`). Nothing is applied without Pro.
- **Advanced repeats:** every N days/weeks/months, chosen weekdays, nth/last weekday of the month, last day of the month, end after N
  times or on a date. Stored as an RRULE-like string in `items.repeat` (`FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=6`); the legacy
  simple values keep working and stay free. NL: `every 2 weeks`, `every mon,wed`, `last friday of the month`, `… for 5 times`,
  `… until dec 31`.

## Roadmap (Pro v2+)
- **GitHub watch:** link a reminder to a repo, branch, PR or tag with live status; auto-complete on merge.
- **Multiple boards/panels:** a panel per project or client.
- **VS Code extension** using the local API; **calendar / week timeline**; **Jira/Linear/GitHub Issues import**.
- **AI quick-add** (paste a Slack/email message); **folder sync** (OneDrive/Dropbox).
- HTTP/remote MCP endpoint so ChatGPT connectors can reach Styky (needs a token-protected localhost HTTP server).

## Pricing notes
- $4.99 early-bird for the first weeks, then $6.99, one-time. One license covers about 3 personal devices.
- Major future versions remain covered ("all future Pro features"). Revisit only if Styky ever needs servers.

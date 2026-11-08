# Styky Privacy Policy

_Last updated: 2026-11-09_

Styky is a desktop reminder app for Windows. It is built to keep your data on your own computer.

## What Styky stores

Your reminders (titles, notes, due times, tags, links) and your settings are saved in a local SQLite database and a small
window-state file in your user profile on your PC. Nothing else is collected.

## What Styky does not do

- **No data leaves your device** (apart from the license check described under *Styky Pro*, if you use Pro). Styky has no accounts, no cloud sync and no servers of its own.
- **No telemetry, analytics, advertising or crash reporting.** Styky does not track how you use it.
- **No third-party sharing.** There is nobody to share data with, because Styky never receives it.

## Network use

- The Microsoft Store version makes no network connections of its own. Updates are delivered by the Microsoft Store.
- The direct-download (GitHub) version only contacts `github.com` when you click **Check for updates** in Settings, to look for
  a newer release. No personal data is sent.

## Styky Pro (optional)

Styky is free. **Styky Pro** is an optional one-time unlock sold by Lemon Squeezy, which acts as the merchant of record: you buy
in your browser on Lemon Squeezy's checkout page, under Lemon Squeezy's own privacy policy. Styky never sees your payment details.

If you enter a Pro license key in Settings, Styky talks to Lemon Squeezy's license service (`api.lemonsqueezy.com`, over HTTPS)
to check it. This is the **only** network traffic Pro adds, and it happens only after you have entered a key:

| When | What is sent |
|---|---|
| You click **Activate** | the license key, and an activation name of the form `Styky 1a2b3c4d` (a random id generated on your PC; **not** your PC name or user name) |
| About once a week, when Styky is running (and at start if a check is due) | the license key and the activation id Lemon Squeezy gave back |
| You click **Remove license** (or activate a different key) | the license key and the activation id, to free the slot |

Like any internet request, Lemon Squeezy's servers also see your IP address and the app's HTTP request headers. Lemon Squeezy's reply
(license status, and the buyer's name, which Styky keeps locally to say "Thank you") is stored in the local database on your PC, together
with the key. The key is not included in Export files. Pro still works offline: Styky tolerates 30 days without a successful check.

Nothing else leaves your PC: not your reminders, not your settings, not usage data. The local API, the `styky` command and the MCP
connector only talk to the Styky app on your own computer through a Windows named pipe that only your Windows user can open; an AI app
you connect through MCP can read and change your reminders (as far as your license allows), so connect only apps you trust.

## Exports and backups

Settings → Data → **Export** writes your reminders to a JSON file in a location you choose. That file is yours; Styky does
not upload it anywhere.

## Notifications and startup

Styky shows Windows notifications for your own reminders and can start when you sign in to Windows. You can turn startup off in
Styky's Settings or in Windows Task Manager → Startup apps.

## Deleting your data

Uninstalling Styky from Windows (Settings → Apps) removes the Microsoft Store version's data. For the direct-download version,
delete the folder `%APPDATA%\com.styky.app` after uninstalling.

## Children

Styky does not collect personal information from anyone, including children.

## Changes and contact

If this policy changes, the new version will be published at this address with an updated date. Questions or concerns:
open an issue at <https://github.com/omorShahriar/styky/issues>.

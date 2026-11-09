# Publishing Styky to the Microsoft Store

Step-by-step for the owner. The Store flavor is the same app packaged as **MSIX** (`scripts/build-msix.ps1`). Microsoft signs
Store MSIX packages for free, so no code-signing certificate is needed. The NSIS/GitHub build is unchanged (see
[RELEASING.md](RELEASING.md)).

## 1. Partner Center account ✅ done

The developer account is registered and the name **Styky** is reserved.

- **Store ID:** `9NZ2ZDFQJLB5`
- **Store link** (live once published): <https://apps.microsoft.com/detail/9NZ2ZDFQJLB5>

## 2. Product identity ✅ done

These come from Partner Center → the product → **Product management → Product identity**. They're public package metadata,
not secrets, so they're baked into `build-msix.ps1 -Store`.

| Partner Center field | Value |
|---|---|
| Package/Identity/Name | `omor.dev.Styky` |
| Package/Identity/Publisher | `CN=150ED2D0-F7B0-4A9D-B63F-68D0C7FDD60C` |
| Package/Properties/PublisherDisplayName | `omor.dev` |
| Package Family Name (PFN) | `omor.dev.Styky_d017wbhzahrzj` |

The PFN is what Windows uses for the installed Store app. For example, toasts show under the AUMID
`omor.dev.Styky_d017wbhzahrzj!Styky`, and its `%APPDATA%\com.styky.app` data is redirected to
`%LOCALAPPDATA%\Packages\omor.dev.Styky_d017wbhzahrzj\LocalCache\Roaming\com.styky.app\`.

## 3. Build the package

Prerequisites: the normal build tools (Node, pnpm, Rust, VS Build Tools) plus the Windows 10/11 SDK. The script installs the SDK
through `winget` if `makeappx.exe` is missing.

```powershell
.\scripts\build-msix.ps1 -Store
```

`-Store` fills in the identity above and builds **unsigned**, which is what Partner Center expects. Add `-SkipBuild` to reuse an
existing `src-tauri\target\release\styky.exe`, but only if no `.rs`/`.ts` file changed since that build.

Output: `dist-msix\Styky_<version>_x64.msix`. The version comes from `src-tauri/tauri.conf.json` with a `.0` appended
(`0.1.0` becomes `0.1.0.0`; the Store reserves the 4th part and every submission must have a higher version than the last).

Unsigned is right for a Store upload: the Store signs the package with its own certificate after certification.
Without `-Store`/`-Unsigned` the script signs with a throwaway self-signed certificate for local testing only (it generates the
cert, signs, and deletes the private key, so there is nothing to keep safe or commit). If you have changed a `.rs`/`.ts`
file since the last build, don't pass `-SkipBuild`.

**Local test before you submit** (optional but recommended): run the script with no switches, which uses a `Styky.LocalTest`
identity and a throwaway certificate. Then either turn on **Developer Mode** (Settings → System → Advanced → For developers),
or in an **admin** PowerShell trust the printed `.cer`, and install:

```powershell
Import-Certificate -FilePath dist-msix\styky-test.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople
Add-AppxPackage dist-msix\Styky_0.1.0.0_x64.msix
```

Remove it with `Get-AppxPackage Styky.LocalTest | Remove-AppxPackage`. Note: do not run the Store build and the NSIS build at
the same time; they share the same app identifier and single-instance lock and the same quick-add hotkey.

## 4. Submit

1. In Partner Center open the product → **Start your submission**.
2. **Pricing and availability:** Markets: all (or your choice). Price: **Free**. Visibility: Public.
3. **Properties:** Category **Productivity**. Privacy policy URL:
   `https://github.com/omorShahriar/styky/blob/main/docs/PRIVACY.md`. Website: `https://styky.pages.dev/`.
   Support contact: `https://github.com/omorShahriar/styky/issues`. System requirements: leave defaults.
   **Product declarations:** don't tick "uses Microsoft Store in-app purchase". Once Pro ships, tick the declaration that the
   app **uses a third-party commerce system** for in-app purchases (Lemon Squeezy, policy 10.8). Non-game apps keep 100% of
   that revenue. Until then there's nothing to declare.
4. **Age ratings:** complete the questionnaire (IARC). Styky has no violence, user-generated sharing, location or
   web browsing, so every answer is "No". The v0.1.0 package has no purchases. The result should be **PEGI 3 / ESRB Everyone / 3+**.
   When a Pro-enabled version ships, update the questionnaire's in-app purchases answer to "Yes".
5. **Packages:** upload `Styky_<version>_x64.msix`. Partner Center validates the Identity values against your reservation; a
   mismatch is rejected immediately with the expected value, so re-run the script with the right one.
6. **Store listings (en-US):** fill in the fields below.
7. **Notes for certification** (shown only to testers): _"Styky is a desktop reminder widget. After launch, a panel appears in the
   top-right of the desktop and a tray icon is added. Press Ctrl+Alt+N for quick add. It declares `runFullTrust` because it is a
   desktop (Win32/WebView2) app. It requires the Microsoft Edge WebView2 Runtime, which is part of Windows 11."_
8. **Submit to the Store.** Certification usually takes 1–3 business days. When it publishes, the **Store link** shows in
   Partner Center; update the "Microsoft Store: coming soon" line in the README.

### Required listing fields

| Field | Value |
|---|---|
| Description | see below (≤10,000 chars) |
| What's new in this version | _First release._ |
| Screenshots | **At least 1** (up to 10), PNG, 1366×768 or larger. Recommended 1920×1080 (16:9) |
| Store logos (optional but recommended) | 300×300 app tile logo (1:1, required for the Store tile) and 1080×1080 (1:1), PNG, from `assets/icon.svg` |
| Search terms (optional) | reminders, sticky notes, desktop widget, to-do, tasks |
| Copyright and trademark info | © 2026 Omor Shahriar. MIT License. |
| Website / Support contact | `https://styky.pages.dev/` and `https://github.com/omorShahriar/styky/issues` |

Screenshot ideas: (1) the panel on a desktop with a few reminders (dark), (2) the quick-add bar with parsed chips,
(3) a reminder toast with the Snooze dropdown, (4) an expanded reminder with a markdown checklist, (5) Settings, (6) light theme.
Take them at 1920×1080 display scale 100%.

## Ready-to-paste Store description

**Short tagline:** The reminders you can't afford to forget, always on your desktop.

**Description:**

> Styky is a small, glassy Windows 11 widget for those one-off but crucial things: release the build on Friday, rotate the API
> keys, reply to the client before 5. It sits on your desktop behind your windows, nudges you with a native notification when
> something is due, and stays out of the way the rest of the time.
>
> Always visible. One frosted-glass panel lives on the desktop layer, behind your apps, so you see it whenever you glance at your
> desktop, even after Win+D.
>
> Capture in two seconds. Press Ctrl+Alt+N anywhere, type "Release v2.3 tomorrow 5pm #api !!", hit Enter. Done.
>
> Nudges, not nagging. A native Windows notification fires on time, with Done and Snooze right on it. Missed reminders while your PC
> was off or asleep are summarized in a single notification, not a flood.
>
> Local-first and private. No account, no cloud, no telemetry. Your reminders stay in a database on your PC, with JSON
> export and import for backups.

**Feature bullets** (up to 20, one per line):

- Desktop widget: a resizable Acrylic glass panel behind your windows that follows light/dark mode and your accent color
- Global quick-add hotkey (Ctrl+Alt+N) with natural-language dates, #tags, !! priority, repeats and links
- Native notifications with a Snooze dropdown (10 minutes, 1 hour, this evening, tomorrow, custom) and Done
- Markdown notes with checklists you can tick right in the widget
- Repeating reminders: daily, weekdays, weekly, monthly
- Developer friendly: branch chips that copy on click, link field, project tags
- Keyboard-first: navigate, complete, snooze, edit and delete without the mouse
- Starts with Windows (turn it off in Settings or Task Manager)
- Private by design: no account, no cloud, no telemetry
- JSON export and import for backups

## After the first release

- Bump the version in `src-tauri/tauri.conf.json`, `package.json`, `src-tauri/Cargo.toml`, rebuild with the same Identity values
  and create a **new submission** in the same product (**Update**), uploading the new `.msix`.
- Store users do not use Styky's built-in updater: it is hidden in the Store flavor.
- The Store version keeps its data separately from the NSIS version (`%APPDATA%\com.styky.app` is virtualized per package,
  see `docs/NOTES.md` → Store (MSIX) notes). Users switching builds can move reminders with Settings → Export / Import.
- **Styky Pro** in the Store build uses the same Lemon Squeezy license key as the GitHub build, not `Windows.Services.Store`
  (see [PRO.md](PRO.md)). The listing stays **Free**; Pro is an optional unlock from the original author, which keeps it clear of
  policy 10.8.7 on charging for freely available open-source software. When the Pro version is submitted, tick the third-party
  commerce declaration and update the age-rating purchase answer (see step 4 above).

## Listing copy for v0.2.0 (developer-first)

For the v0.2.0 submission. The v0.1.0 copy above stays as the record of the first release.

> **Check before pasting:** the Store (MSIX) build does **not** include the `styky` CLI or the MCP server (they need an
> `appExecutionAlias`, see [PRO.md](PRO.md)). This copy therefore only promises them as part of the GitHub installer. If you add
> the alias to the Store package, move the CLI/MCP bullets up and drop the "GitHub installer" sentence. Also tick the third-party
> commerce declaration and answer "Yes" for in-app purchases (step 4), because Pro is now in the build.

**Short description** (270 characters or fewer):

> Developer-friendly reminders: add with Ctrl+Alt+N, get native Windows notifications with Done and Snooze, see them in a glass desktop panel. Free and open source. Optional one-time Pro adds Critical reminders that keep notifying, pre-alerts and themes.

**Description:**

> Styky is a small Windows 11 reminder widget for the one-off things you cannot afford to forget: deploy on Friday, rotate the API keys, check CI after the push, reply to the client before 5.
>
> Add a reminder in two seconds. Press Ctrl+Alt+N anywhere, type "Deploy api fri 5pm #infra !!" and press Enter. Dates, #tags, @branch chips, links, priority and repeats are all parsed from the sentence.
>
> Native notifications that do their job. A Windows notification fires on time with Done and a Snooze dropdown (10 minutes, 1 hour, this evening, tomorrow, custom). Reminders you missed while the PC was off arrive as one summary, not a flood.
>
> Always in sight. A resizable frosted-glass panel sits on the desktop layer behind your windows and follows light and dark mode and your accent color.
>
> Built for developers. Branch chips that copy on click, link fields, project tags, markdown notes with tickable checklists, and full keyboard control. The GitHub installer also adds a styky command line and an MCP server, so Claude Desktop, Claude Code and other MCP clients can add and list your reminders (free) and complete, snooze and edit them (Pro).
>
> Local-first. No account, no cloud, no telemetry. Reminders live in a database on your PC, with JSON export and import. Styky is open source under the MIT license.
>
> Styky Pro is an optional one-time unlock, bought outside the Store with a license key. It adds Critical reminders that re-notify until you act, pre-alerts from 5 minutes to 1 day before, advanced repeats (every 2 weeks, Mon/Wed/Fri, the last Friday of the month), themes, and the Pro commands of the CLI and MCP server. Everything else stays free.

**Feature bullets** (one per line):

- Quick add anywhere with Ctrl+Alt+N: natural-language dates, #tags, @branch, !! priority, repeats, links
- Native notifications with Done and a Snooze dropdown; missed reminders arrive as one summary
- Desktop widget: a resizable Acrylic glass panel behind your windows, light and dark
- Developer touches: branch chips that copy on click, links, project tags, markdown checklists
- Keyboard-first: navigate, complete, snooze, edit and delete without the mouse
- Local-first and open source (MIT): no account, no cloud, no telemetry, JSON export and import
- Pro (optional, one-time): Critical reminders that keep notifying, pre-alerts, advanced repeats, themes
- Command line and MCP server for Claude in the GitHub installer (add and list free, the rest Pro)
- Starts with Windows (turn it off in Settings or Task Manager)

**Search terms** (7 terms, 21 words or fewer in total):

`developer reminders` · `command line reminders` · `desktop widget` · `sticky notes` · `quick add` · `to-do list` · `Claude MCP`

**What's new in this version:**

> Styky Pro: Critical reminders that keep notifying until you act, pre-alerts, advanced repeats and themes. The GitHub edition also gains a styky command line and an MCP server for Claude.

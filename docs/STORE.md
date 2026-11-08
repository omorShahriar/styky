# Publishing Styky to the Microsoft Store

Step-by-step for the owner. The Store flavor is the same app packaged as **MSIX** (`scripts/build-msix.ps1`). Microsoft signs
Store MSIX packages for free, so no code-signing certificate is needed. The NSIS/GitHub build is unchanged (see
[RELEASING.md](RELEASING.md)).

## 1. Partner Center account (one-time)

1. Go to <https://partner.microsoft.com/dashboard/registration/developer> and sign in with a Microsoft account.
2. Choose **Individual** account type, fill in your name, address and contact details, and pay the one-time registration fee
   (about US $19 depending on country). Verification can take from minutes to a few days.
3. In **Apps and games**, click **New product → MSIX or PWA app** and **reserve the name `Styky`**. Reservations last 3 months
   until you submit; if the name is taken, pick a variation (e.g. "Styky Reminders"). The package display name does not need
   to match the reserved name exactly, but the Store listing uses the reserved one.

## 2. Copy the Identity values

Open the product → **Product management → Product identity**. Copy three values:

| Partner Center field | `build-msix.ps1` parameter | Looks like |
|---|---|---|
| Package/Identity/Name | `-IdentityName` | `12345YourName.Styky` |
| Package/Identity/Publisher | `-Publisher` | `CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX` |
| Package/Properties/PublisherDisplayName | `-PublisherDisplayName` | `Omor Shahriar` |

## 3. Build the package

Prerequisites: the normal build tools (Node, pnpm, Rust, VS Build Tools) plus the Windows 10/11 SDK. The script installs the SDK
through `winget` if `makeappx.exe` is missing.

```powershell
.\scripts\build-msix.ps1 `
  -IdentityName "12345YourName.Styky" `
  -Publisher "CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX" `
  -PublisherDisplayName "Omor Shahriar" `
  -Unsigned
```

Output: `dist-msix\Styky_<version>_x64.msix`. The version comes from `src-tauri/tauri.conf.json` with a `.0` appended
(`0.1.0` becomes `0.1.0.0`; the Store reserves the 4th part and every submission must have a higher version than the last).

`-Unsigned` is right for a Store upload: the Store signs the package with its own certificate after certification.
Without `-Unsigned` the script signs with a throwaway self-signed certificate for local testing only (it generates the
cert, signs, and deletes the private key, so there is nothing to keep safe or commit). If you have changed a `.rs`/`.ts`
file since the last build, don't pass `-SkipBuild`.

**Local test before you submit** (optional but recommended): run the script without `-IdentityName`/`-Publisher`/`-Unsigned`,
then in an **admin** PowerShell trust the printed `.cer` and install:

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
   `https://github.com/omorShahriar/styky/blob/main/docs/PRIVACY.md` (make sure the repo is public and this file is pushed).
   System requirements: leave defaults. "Declare that this app can run on Windows desktop" only.
4. **Age ratings:** complete the questionnaire (IARC). Styky has no violence, user-generated sharing, purchases, location or
   web access, so every answer is "No"; the result should be **PEGI 3 / ESRB Everyone / 3+**.
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
| Website / Support contact | `https://github.com/omorShahriar/styky` and `https://github.com/omorShahriar/styky/issues` |

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
- A paid "Pro" unlock later needs in-app purchase (Windows.Services.Store) work in the app; MSIX packaging is the prerequisite
  and is now in place.

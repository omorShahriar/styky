# Releasing Styky

Maintainer notes for cutting a release. Installed copies check
`https://github.com/omorShahriar/styky/releases/latest/download/latest.json` for updates, and every update must be signed
with the Styky updater key.

> The Microsoft Store (MSIX) build is covered at the end of this file.

## One-time setup

- The updater **private key** lives at `%USERPROFILE%\.tauri\styky.key`, outside the repo. `*.key` is git-ignored.
  **Back it up.** If it is lost, installed copies can never be updated again (they only trust the matching public key in
  `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`).
- A new key (only if starting over) is made with `pnpm tauri signer generate -w %USERPROFILE%\.tauri\styky.key`. Then put the new
  `.pub` contents into `tauri.conf.json`.

## Cutting a release

1. Bump `version` in `src-tauri/tauri.conf.json`, `package.json` and `src-tauri/Cargo.toml` (and the `styky` entry in
   `src-tauri/Cargo.lock`, which `cargo` rewrites on the next build). Add the version's section to `CHANGELOG.md` and copy it to
   `docs/release-notes/v<version>.md` (the GitHub release notes). Then commit. (`site/` has its own version and reads the release
   data at build time; leave it alone.)
2. Run `.\scripts\build.ps1`. It loads the key into `TAURI_SIGNING_PRIVATE_KEY` and runs `pnpm tauri build`. Output is in
   `src-tauri\target\release\bundle\nsis\`:
   - `Styky_<version>_x64-setup.exe`
   - `Styky_<version>_x64-setup.exe.sig`
3. Generate `latest.json` with `node scripts/latest-json.mjs <version> "<release notes>"`, which writes it to the bundle folder.
   It also writes `Styky_x64-setup.exe`, an unversioned copy of the installer, and ends by listing the four files to upload:
   the versioned installer, `Styky_x64-setup.exe`, the `.sig` and `latest.json`. Upload `Styky_x64-setup.exe` to **every** release
   (step 4): the website links to `https://github.com/omorShahriar/styky/releases/latest/download/Styky_x64-setup.exe`, so the
   download button never needs a redeploy.
4. Tag and publish:

   ```powershell
   git tag v<version>; git push origin v<version>
   gh release create v<version> --title "Styky <version>" --notes-file docs\release-notes\v<version>.md `
     src-tauri\target\release\bundle\nsis\Styky_<version>_x64-setup.exe `
     src-tauri\target\release\bundle\nsis\Styky_x64-setup.exe `
     src-tauri\target\release\bundle\nsis\Styky_<version>_x64-setup.exe.sig `
     src-tauri\target\release\bundle\nsis\latest.json
   ```

`releases/latest/download/latest.json` always resolves to the newest non-prerelease, so installed copies find it automatically.
Mark a release as a *pre-release* if it should not be offered as an update.

### `latest.json` format

```json
{
  "version": "0.2.0",
  "notes": "What changed",
  "pub_date": "2026-01-01T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<contents of Styky_0.2.0_x64-setup.exe.sig>",
      "url": "https://github.com/omorShahriar/styky/releases/download/v0.2.0/Styky_0.2.0_x64-setup.exe"
    }
  }
}
```

## Microsoft Store (MSIX) build

The Store flavor is the same app packaged as **MSIX** by `scripts/build-msix.ps1`. Microsoft signs Store MSIX packages, so no
code-signing certificate is involved. The NSIS/GitHub build above is unchanged. The package has no CLI or MCP server yet.

### Package identity

Public package metadata (Partner Center → Product management → Product identity). Name and Publisher are written into the manifest
template `packaging/msix/AppxManifest.xml`; `build-msix.ps1 -Store` uses them as they are, and a local-test build swaps in a test
identity:

| Field | Value |
|---|---|
| Package/Identity/Name | `omor.dev.Styky` |
| Package/Identity/Publisher | `CN=150ED2D0-F7B0-4A9D-B63F-68D0C7FDD60C` |
| Package/Properties/PublisherDisplayName | `omor.dev` (build parameter `-PublisherDisplayName`) |
| Package Family Name (PFN) | `omor.dev.Styky_d017wbhzahrzj` |
| Store ID | `9NZ2ZDFQJLB5` |

Windows uses the PFN for the installed app. Toasts show under the AUMID `omor.dev.Styky_d017wbhzahrzj!Styky`, and the app's
`%APPDATA%\com.styky.app` data is redirected by MSIX file virtualization to
`%LOCALAPPDATA%\Packages\omor.dev.Styky_d017wbhzahrzj\LocalCache\Roaming\com.styky.app\`. It is deleted on uninstall and is separate from
the NSIS install's database; Settings → Export / Import moves reminders between the two.

A Partner Center upload is checked against the account's current publisher display name, so build with the
`-PublisherDisplayName` the account has at that moment (default `omor.dev`):
`.\scripts\build-msix.ps1 -Store -PublisherDisplayName "<name>"`.

### Build

Prerequisites: the normal build tools (Node, pnpm, Rust, VS Build Tools) plus the Windows 10/11 SDK. The script installs the SDK
through `winget` if `makeappx.exe` is missing.

```powershell
.\scripts\build-msix.ps1 -Store
```

`-Store` uses the identity above (and refuses `-IdentityName` / `-Publisher`) and builds **unsigned**, which is what Partner Center expects (the Store signs after
certification). Add `-SkipBuild` to reuse an existing `src-tauri\target\release\styky.exe`, but only if no `.rs`/`.ts` file changed since
that build.

Output: `dist-msix\Styky_<version>_x64.msix` (git-ignored). The version is `src-tauri/tauri.conf.json` with a `.0` appended
(`0.2.0` becomes `0.2.0.0`; the Store reserves the 4th part and every submission needs a higher version than the last).

**Local test:** run the script with no switches. It uses a `Styky.LocalTest` identity and a throwaway self-signed certificate
(the private key is deleted right after signing). Then turn on **Developer Mode** (Settings → System → Advanced → For developers),
or in an **admin** PowerShell trust the printed `.cer`, and install:

```powershell
Import-Certificate -FilePath dist-msix\styky-test.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople
Add-AppxPackage dist-msix\Styky_0.2.0.0_x64.msix
```

Remove it with `Get-AppxPackage Styky.LocalTest | Remove-AppxPackage`. Do not run the Store build and the NSIS build at the same
time: they share the app identifier, the single-instance lock and the quick-add hotkey. Never do any of this from inside a Claude
desktop app session (it is MSIX-packaged, so processes it starts see virtualized files and registry).

### What differs in the Store flavor

- The updater plugin is not registered when `store::is_store()` (packaged at runtime, or built with `--features store`), and
  Settings hides the Updates row. Updates come from the Store.
- Launch at login uses the manifest `windows.startupTask` (`StykyStartup`, enabled by default); a user who disabled it in Task
  Manager sees a hint in Settings.
- The manifest declares `runFullTrust` and `MinVersion` 10.0.17763.0 (Windows 10 1809).

### Later versions

Bump the version everywhere (see "Cutting a release"), rebuild with the same identity values and create a new submission in the
same product, uploading the new `.msix`. `docs/NOTES.md` → "Store (MSIX) notes" has the longer record of what was and was not
verified under MSIX.

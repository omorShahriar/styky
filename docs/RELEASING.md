# Releasing Styky

Maintainer notes for cutting a release. Installed copies check
`https://github.com/omorShahriar/styky/releases/latest/download/latest.json` for updates, and every update must be signed
with the Styky updater key.

## One-time setup

- The updater **private key** lives at `%USERPROFILE%\.tauri\styky.key`, outside the repo. `*.key` is git-ignored.
  **Back it up.** If it is lost, installed copies can never be updated again (they only trust the matching public key in
  `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`).
- A new key (only if starting over) is made with `pnpm tauri signer generate -w %USERPROFILE%\.tauri\styky.key`. Then put the new
  `.pub` contents into `tauri.conf.json`.

## Cutting a release

1. Bump `version` in `src-tauri/tauri.conf.json`, `package.json` and `src-tauri/Cargo.toml`, then commit.
2. Run `.\scripts\build.ps1`. It loads the key into `TAURI_SIGNING_PRIVATE_KEY` and runs `pnpm tauri build`. Output is in
   `src-tauri\target\release\bundle\nsis\`:
   - `Styky_<version>_x64-setup.exe`
   - `Styky_<version>_x64-setup.exe.sig`
3. Generate `latest.json` with `node scripts/latest-json.mjs <version> "<release notes>"`, which writes it to the bundle folder.
4. Tag and publish:

   ```powershell
   git tag v<version>; git push origin v<version>
   gh release create v<version> --title "Styky <version>" --notes-file <notes.md> `
     src-tauri\target\release\bundle\nsis\Styky_<version>_x64-setup.exe `
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

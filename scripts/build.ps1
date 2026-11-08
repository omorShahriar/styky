# Builds the signed NSIS installer. The updater signing key lives OUTSIDE the repo.
$ErrorActionPreference = "Stop"
$key = Join-Path $env:USERPROFILE ".tauri\styky.key"
if (-not (Test-Path $key)) { throw "Signing key not found at $key. Run: pnpm tauri signer generate -w $key" }
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw $key
if ($null -eq $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "" }
$cargo = Join-Path $env:USERPROFILE ".cargo\bin"
if ($env:Path -notlike "*$cargo*") { $env:Path += ";$cargo" }
Set-Location (Join-Path $PSScriptRoot "..")
pnpm tauri build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Get-ChildItem "src-tauri\target\release\bundle\nsis" | Select-Object FullName, Length

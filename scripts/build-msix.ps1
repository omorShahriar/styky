<#
.SYNOPSIS
  Builds the Microsoft Store (MSIX) package for x64 from the release build.

.DESCRIPTION
  Tauri has no MSIX bundler, so this stages styky.exe + tile assets + AppxManifest.xml and packs them with makeappx.
  The real Partner Center identity (Name omor.dev.Styky and its Publisher CN) lives in packaging\msix\AppxManifest.xml.
  A -Store build uses it as it is; a local-test build replaces it with a test identity so it never collides with the Store app.
  PublisherDisplayName is a parameter (default "Small Hinge Labs"); it has to match the Partner Center account's publisher display
  name at upload time, so pass -PublisherDisplayName "..." once the account name changes.

  Store upload: .\scripts\build-msix.ps1 -Store               (real identity, unsigned)
  Local test:   .\scripts\build-msix.ps1                      (test identity, self-signed, writes a .cer to trust)
  The Store signs the upload itself, so a Store submission needs no certificate.
#>
param(
  [switch]$Store,                   # use the manifest template's real identity and skip signing
  [string]$IdentityName = "Styky.LocalTest",   # local-test builds only
  [string]$Publisher = "CN=StykyLocalTest",    # local-test builds only (also the throwaway certificate's subject)
  [string]$PublisherDisplayName = "Small Hinge Labs",
  [string]$Version,                 # default: tauri.conf.json version + ".0"
  [switch]$Unsigned,                # skip signing (Store upload)
  [switch]$SkipBuild,               # reuse src-tauri\target\release\styky.exe
  [string]$OutDir
)
$ErrorActionPreference = "Stop"

if ($Store) {
  if ($PSBoundParameters.ContainsKey("IdentityName") -or $PSBoundParameters.ContainsKey("Publisher")) {
    throw "-Store uses the identity in packaging\msix\AppxManifest.xml; -IdentityName and -Publisher are for local-test builds"
  }
  $Unsigned = $true
}
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root
if (-not $OutDir) { $OutDir = Join-Path $root "dist-msix" }
$cargo = Join-Path $env:USERPROFILE ".cargo\bin"
if ($env:Path -notlike "*$cargo*") { $env:Path += ";$cargo" }

if (-not $Version) {
  $v = (Get-Content -Raw "src-tauri\tauri.conf.json" | ConvertFrom-Json).version
  $Version = "$v.0"
}
if ($Version -notmatch '^\d+\.\d+\.\d+\.0$') { throw "Version must be a.b.c.0 (the Store reserves the 4th part); got $Version" }

# --- Windows SDK tools (winget install if missing)
function Find-SdkTool([string]$name) {
  $bin = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
  if (-not (Test-Path $bin)) { return $null }
  Get-ChildItem $bin -Directory | Where-Object Name -match '^10\.' | Sort-Object { [version]$_.Name } -Descending |
    ForEach-Object { Join-Path $_.FullName "x64\$name" } | Where-Object { Test-Path $_ } | Select-Object -First 1
}
$makeappx = Find-SdkTool "makeappx.exe"
if (-not $makeappx) {
  Write-Host "Windows SDK not found; installing via winget..."
  winget install --id Microsoft.WindowsSDK.10.0.26100 -e --accept-source-agreements --accept-package-agreements
  $makeappx = Find-SdkTool "makeappx.exe"
  if (-not $makeappx) { throw "makeappx.exe still not found after installing the Windows SDK" }
}
$makepri = Find-SdkTool "makepri.exe"
$signtool = Find-SdkTool "signtool.exe"

# --- release build (no installer bundles)
if (-not $SkipBuild) {
  $ErrorActionPreference = "Continue"   # native stderr (cargo/tauri progress) must not abort the script
  # The CLI (styky-cli) ships as bin\styky.exe behind the `styky` execution alias, as in the NSIS build.
  cargo build --release --bin styky-cli --manifest-path src-tauri\Cargo.toml
  $code = $LASTEXITCODE
  if ($code -eq 0) {
    pnpm tauri build --no-bundle
    $code = $LASTEXITCODE
  }
  $ErrorActionPreference = "Stop"
  $LASTEXITCODE = $code
  if ($LASTEXITCODE -ne 0) { throw "release build failed" }
}
$exe = "src-tauri\target\release\styky.exe"
$cli = "src-tauri\target\release\styky-cli.exe"
foreach ($f in $exe, $cli) { if (-not (Test-Path $f)) { throw "$f not found" } }

# --- stage
$stage = Join-Path $OutDir "stage"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force (Join-Path $stage "Assets") | Out-Null
New-Item -ItemType Directory -Force (Join-Path $stage "bin") | Out-Null
Copy-Item $exe (Join-Path $stage "styky.exe")
Copy-Item $cli (Join-Path $stage "bin\styky.exe")

# Tile assets are resized from the app icon; scale variants are resolved through resources.pri.
Add-Type -AssemblyName System.Drawing
$src = [System.Drawing.Image]::FromFile((Resolve-Path "src-tauri\icons\icon.png"))
function Save-Png([string]$path, [int]$size) {
  $bmp = New-Object System.Drawing.Bitmap $size, $size
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.InterpolationMode = "HighQualityBicubic"; $g.SmoothingMode = "HighQuality"; $g.PixelOffsetMode = "HighQuality"
  $g.DrawImage($src, 0, 0, $size, $size); $g.Dispose()
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
}
$assets = Join-Path $stage "Assets"
foreach ($s in 100, 125, 150, 200, 400) {
  $f = $s / 100
  Save-Png (Join-Path $assets "Square44x44Logo.scale-$s.png") ([int](44 * $f))
  Save-Png (Join-Path $assets "Square150x150Logo.scale-$s.png") ([int](150 * $f))
  Save-Png (Join-Path $assets "StoreLogo.scale-$s.png") ([int](50 * $f))
}
foreach ($t in 16, 24, 32, 48, 256) {
  Save-Png (Join-Path $assets "Square44x44Logo.targetsize-$t.png") $t
  Save-Png (Join-Path $assets "Square44x44Logo.targetsize-${t}_altform-unplated.png") $t
}
$src.Dispose()

$manifest = Get-Content -Raw "packaging\msix\AppxManifest.xml"
if (-not $Store) {
  # Local test: a different identity, so the test package never replaces (or is replaced by) the Store app.
  $manifest = [regex]::Replace($manifest, '(<Identity\s+Name=")[^"]*(" Publisher=")[^"]*(")', {
    param($m) $m.Groups[1].Value + $IdentityName + $m.Groups[2].Value + $Publisher + $m.Groups[3].Value
  })
}
$manifest = $manifest.Replace("{{PUBLISHER_DISPLAY_NAME}}", [Security.SecurityElement]::Escape($PublisherDisplayName)).Replace("{{VERSION}}", $Version)
if ($manifest -match '\{\{') { throw "AppxManifest.xml still has an unreplaced {{token}}" }
Write-Host ("Identity: " + ([regex]::Match($manifest, '<Identity [^>]*>').Value))
[IO.File]::WriteAllText((Join-Path $stage "AppxManifest.xml"), $manifest, (New-Object Text.UTF8Encoding $false))

# resources.pri so Windows can pick the right scale/targetsize of each tile asset
$pricfg = Join-Path $OutDir "priconfig.xml"
& $makepri createconfig /cf $pricfg /dq en-US /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makepri createconfig failed" }
& $makepri new /pr $stage /cf $pricfg /mn (Join-Path $stage "AppxManifest.xml") /of (Join-Path $stage "resources.pri") /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makepri new failed" }

# --- pack
$msix = Join-Path $OutDir "Styky_${Version}_x64.msix"
if (Test-Path $msix) { Remove-Item $msix -Force }
& $makeappx pack /d $stage /p $msix /o
if ($LASTEXITCODE -ne 0) { throw "makeappx pack failed" }

# --- sign with a throwaway self-signed cert (local testing only; never commit the pfx)
if (-not $Unsigned) {
  $cert = New-SelfSignedCertificate -Type Custom -Subject $Publisher -KeyUsage DigitalSignature -FriendlyName "Styky local test" `
    -CertStoreLocation "Cert:\CurrentUser\My" -KeyExportPolicy Exportable `
    -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
  $pw = ConvertTo-SecureString -String ([guid]::NewGuid().ToString()) -Force -AsPlainText
  $pfx = Join-Path $OutDir "styky-test.pfx"
  Export-PfxCertificate -Cert $cert -FilePath $pfx -Password $pw | Out-Null
  Export-Certificate -Cert $cert -FilePath (Join-Path $OutDir "styky-test.cer") | Out-Null
  $plain = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($pw))
  & $signtool sign /fd SHA256 /a /f $pfx /p $plain $msix
  if ($LASTEXITCODE -ne 0) { throw "signtool failed" }
  Remove-Item $pfx -Force
  Remove-Item $cert.PSPath -Force   # drop the private key from the cert store too
  Write-Host "Signed with a throwaway cert. To install locally, trust it first:"
  Write-Host "  Import-Certificate -FilePath $OutDir\styky-test.cer -CertStoreLocation Cert:\CurrentUser\TrustedPeople"
}
Write-Host "Built $msix"
Get-Item $msix | Select-Object FullName, Length

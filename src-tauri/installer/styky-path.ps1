# Adds/removes a directory on the *user* PATH (HKCU\Environment\Path), keeping the value type (REG_EXPAND_SZ)
# and every other entry. Called by the NSIS installer hooks; also usable on its own.
#   styky-path.ps1 -Action add|remove -Dir "C:\Users\me\AppData\Local\Styky\bin"
# -SubKey is only for tests (defaults to the real Environment key); -NoBroadcast skips the settings-change broadcast.
param(
  [Parameter(Mandatory = $true)][ValidateSet('add', 'remove')][string]$Action,
  [Parameter(Mandatory = $true)][string]$Dir,
  [string]$SubKey = 'Environment',
  [switch]$NoBroadcast
)
$ErrorActionPreference = 'Stop'
$norm = { param($s) $s.Trim().TrimEnd('\') }
$key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($SubKey)
try {
  $raw = [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
  $kind = if ($key.GetValueNames() -contains 'Path') { $key.GetValueKind('Path') } else { [Microsoft.Win32.RegistryValueKind]::ExpandString }
  $parts = @($raw -split ';' | Where-Object { $_.Trim() -ne '' -and (& $norm $_) -ine (& $norm $Dir) })
  if ($Action -eq 'add') { $parts += $Dir }
  $new = $parts -join ';'
  if ($new -ne $raw) { $key.SetValue('Path', $new, $kind) }
} finally {
  $key.Close()
}
if (-not $NoBroadcast) {
  # Setting and clearing a throwaway variable makes Windows broadcast WM_SETTINGCHANGE, so new shells see the change.
  [Environment]::SetEnvironmentVariable('STYKY_PATH_NOTIFY', '1', 'User')
  [Environment]::SetEnvironmentVariable('STYKY_PATH_NOTIFY', $null, 'User')
}

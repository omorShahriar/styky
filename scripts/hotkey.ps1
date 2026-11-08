# Dev helper: synthesize a key chord, e.g.  powershell -File scripts/hotkey.ps1 ctrl+alt+n   |  ... enter  |  ... escape
param([string]$Chord = "ctrl+alt+n")
Add-Type -TypeDefinition 'using System.Runtime.InteropServices; public class K2 { [DllImport("user32.dll")] public static extern void keybd_event(byte b, byte s, uint f, int e); }'
$map = @{ ctrl = 0x11; alt = 0x12; shift = 0x10; win = 0x5B; enter = 0x0D; escape = 0x1B }
$keys = $Chord.ToLower().Split('+') | % { if ($map.ContainsKey($_)) { $map[$_] } else { [byte][char]$_.ToUpper() } }
foreach ($k in $keys) { [K2]::keybd_event($k, 0, 0, 0) }
[array]::Reverse($keys)
foreach ($k in $keys) { [K2]::keybd_event($k, 0, 2, 0) }

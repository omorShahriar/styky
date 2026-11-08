# Dev helper: open the Windows 11 notification center (Win+N) so toast buttons are reachable via UI Automation.
Add-Type -TypeDefinition 'using System.Runtime.InteropServices; public class K { [DllImport("user32.dll")] public static extern void keybd_event(byte b, byte s, uint f, int e); }'
[K]::keybd_event(0x5B,0,0,0); [K]::keybd_event(0x4E,0,0,0); [K]::keybd_event(0x4E,0,2,0); [K]::keybd_event(0x5B,0,2,0)

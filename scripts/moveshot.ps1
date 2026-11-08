# Dev helper: move Styky's panel to X,Y (keeping its size), then screenshot that region to -Out.
param([int]$X, [int]$Y, [string]$Out)
Add-Type @"
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public class M {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int w, int hh, uint f);
  public static IntPtr Panel(uint pid) {
    IntPtr r = IntPtr.Zero;
    EnumWindows((h, _) => { uint p; GetWindowThreadProcessId(h, out p); var t = new StringBuilder(32); GetWindowText(h, t, 32); if (p == pid && t.ToString() == "Styky") r = h; return true; }, IntPtr.Zero);
    return r;
  }
}
"@
$h = [M]::Panel([uint32](Get-Process styky | select -First 1).Id)
[void][M]::SetWindowPos($h, [IntPtr]1, $X, $Y, 0, 0, 0x0011)  # NOSIZE | NOACTIVATE, HWND_BOTTOM
Start-Sleep 1
Add-Type -AssemblyName System.Drawing
$bmp = New-Object Drawing.Bitmap 2560,1440; [Drawing.Graphics]::FromImage($bmp).CopyFromScreen(0,0,0,0,$bmp.Size)
$bmp.Clone([Drawing.Rectangle]::new([Math]::Max(0,$X-60),[Math]::Max(0,$Y-24),500,640),$bmp.PixelFormat).Save($Out)

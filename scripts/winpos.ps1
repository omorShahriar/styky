# Dev helper: print the state of Styky's panel window (visible / iconic / rect / visible windows above it in z-order).
Add-Type @"
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public class W {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int i);
  [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint c);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  public static List<IntPtr> Find(uint pid) {
    var l = new List<IntPtr>();
    EnumWindows((h, _) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid) l.Add(h); return true; }, IntPtr.Zero);
    return l;
  }
}
"@
$p = Get-Process styky -ErrorAction SilentlyContinue | select -First 1
if (-not $p) { "no process"; exit }
foreach ($h in [W]::Find([uint32]$p.Id)) {
  $r = New-Object W+RECT; [void][W]::GetWindowRect($h, [ref]$r)
  if (($r.R - $r.L) -lt 100) { continue }
  $t = New-Object Text.StringBuilder 64; [void][W]::GetWindowText($h, $t, 64)
  $z = 0; $c = $h; while (($c = [W]::GetWindow($c, 3)) -ne [IntPtr]::Zero) { if ([W]::IsWindowVisible($c)) { $z++ } }
  "title='{0}' visible={1} iconic={2} rect=({3},{4},{5},{6}) visibleWindowsAbove={7}" -f $t, [W]::IsWindowVisible($h), [W]::IsIconic($h), $r.L,$r.T,$r.R,$r.B,$z
}

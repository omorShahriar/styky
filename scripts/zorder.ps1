# Dev helper: list top-level windows from the bottom of the z-order up (class, visible, title) for the lowest N.
param([int]$N = 12, [switch]$Top)
Add-Type @"
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public class Z {
  [DllImport("user32.dll")] static extern IntPtr GetTopWindow(IntPtr h);
  [DllImport("user32.dll")] static extern IntPtr GetWindow(IntPtr h, uint c);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out int[] r);
  public static List<string> All() {
    var l = new List<string>();
    for (IntPtr h = GetTopWindow(IntPtr.Zero); h != IntPtr.Zero; h = GetWindow(h, 2)) {
      var c = new StringBuilder(64); GetClassName(h, c, 64); var t = new StringBuilder(48); GetWindowText(h, t, 48);
      l.Add(string.Format("{0} vis={1} cls={2} title='{3}'", h.ToInt64().ToString("x"), IsWindowVisible(h), c, t));
    }
    return l;
  }
}
"@
$a = [Z]::All(); if ($Top) { $a | select -First $N } else { $a | select -Last $N }

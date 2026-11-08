# Dev helper: press Win+D n times, after each print panel state and save a screenshot of the panel region.
param([int]$N = 2, [string]$Prefix = "D:\dev\styky\.wd")
Add-Type -Namespace K -Name N -MemberDefinition '[DllImport("user32.dll")] public static extern void keybd_event(byte b, byte s, uint f, UIntPtr e);'
Add-Type -AssemblyName System.Drawing
function Shot($n) { $bmp = New-Object Drawing.Bitmap 2560,1440; [Drawing.Graphics]::FromImage($bmp).CopyFromScreen(0,0,0,0,$bmp.Size); $bmp.Clone([Drawing.Rectangle]::new(2100,0,460,640),$bmp.PixelFormat).Save("$Prefix$n.png") }
Shot 0; "state0:"; powershell -NoProfile -File "$PSScriptRoot\winpos.ps1" | select -Last 1
for ($i = 1; $i -le $N; $i++) {
  [K.N]::keybd_event(0x5B,0,0,[UIntPtr]::Zero); [K.N]::keybd_event(0x44,0,0,[UIntPtr]::Zero); [K.N]::keybd_event(0x44,0,2,[UIntPtr]::Zero); [K.N]::keybd_event(0x5B,0,2,[UIntPtr]::Zero)
  Start-Sleep 2; Shot $i; "after Win+D #${i}:"; powershell -NoProfile -File "$PSScriptRoot\winpos.ps1" | select -Last 1
}

# Dev helper: click a button on the visible Windows toast through UI Automation.
#   pwsh scripts/toastclick.ps1 -Name Done
#   pwsh scripts/toastclick.ps1 -Name Snooze -Select "1 hour"   (picks the dropdown value first)
param([string]$Name = "Done", [string]$Select = "")
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$root = [System.Windows.Automation.AutomationElement]::RootElement
function Find($name, $type) {
  $c1 = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, $name)
  $c2 = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, $type)
  $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.AndCondition($c1, $c2)))
}
if ($Select) {
  $combo = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,
    (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ComboBox)))
  if (-not $combo) { throw "no dropdown on screen" }
  $combo.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern).Expand()
  Start-Sleep -Milliseconds 400
  $opt = Find $Select ([System.Windows.Automation.ControlType]::ListItem)
  if (-not $opt) { throw "no option '$Select'" }
  $opt.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select()
  Start-Sleep -Milliseconds 300
}
$btn = Find $Name ([System.Windows.Automation.ControlType]::Button)
if (-not $btn) { throw "no '$Name' button on screen" }
$btn.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
"clicked $Name"

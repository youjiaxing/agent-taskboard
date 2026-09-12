param(
  [Parameter(Mandatory = $true)]
  [int]$ProcessId,
  [Parameter(Mandatory = $true)]
  [string]$EvidenceDir
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class AgentTaskboardNativeUi {
  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern bool SetCursorPos(int x, int y);

  [DllImport("user32.dll")]
  public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);

  public static void Click(int x, int y, bool right) {
    SetCursorPos(x, y);
    uint down = right ? 0x0008u : 0x0002u;
    uint up = right ? 0x0010u : 0x0004u;
    mouse_event(down, 0, 0, 0, UIntPtr.Zero);
    mouse_event(up, 0, 0, 0, UIntPtr.Zero);
  }
}
"@

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null

function Save-Screen([string]$Name) {
  $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  try {
    $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $bitmap.Save((Join-Path $EvidenceDir $Name), [System.Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $graphics.Dispose()
    $bitmap.Dispose()
  }
}

function Wait-Until([scriptblock]$Condition, [string]$Failure, [int]$Attempts = 60) {
  for ($attempt = 0; $attempt -lt $Attempts; $attempt += 1) {
    if (& $Condition) { return }
    Start-Sleep -Milliseconds 500
  }
  throw $Failure
}

function All-Desktop-Elements {
  [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
    [System.Windows.Automation.TreeScope]::Descendants,
    [System.Windows.Automation.Condition]::TrueCondition
  )
}

function Find-Visible-Element([string]$NamePattern) {
  foreach ($element in (All-Desktop-Elements)) {
    try {
      if (-not $element.Current.IsOffscreen -and $element.Current.Name -match $NamePattern) {
        return $element
      }
    } catch {}
  }
  return $null
}

function Find-Tray-Element {
  foreach ($element in (All-Desktop-Elements)) {
    try {
      $controlType = $element.Current.ControlType.ProgrammaticName
      if (-not $element.Current.IsOffscreen -and
          $element.Current.Name -eq "Agent Taskboard" -and
          $controlType -ne "ControlType.Window") {
        return $element
      }
    } catch {}
  }
  return $null
}

function Click-Element($Element, [bool]$Right = $false) {
  $rectangle = $Element.Current.BoundingRectangle
  if ($rectangle.Width -le 0 -or $rectangle.Height -le 0) {
    throw "element '$($Element.Current.Name)' has no clickable rectangle"
  }
  $x = [int]($rectangle.Left + ($rectangle.Width / 2))
  $y = [int]($rectangle.Top + ($rectangle.Height / 2))
  [AgentTaskboardNativeUi]::Click($x, $y, $Right)
}

function Invoke-Element($Element) {
  $pattern = $null
  if ($Element.TryGetCurrentPattern(
      [System.Windows.Automation.InvokePattern]::Pattern,
      [ref]$pattern
  )) {
    $pattern.Invoke()
  } else {
    Click-Element $Element
  }
}

function Toggle-Element($Element) {
  $pattern = $null
  if (-not $Element.TryGetCurrentPattern(
      [System.Windows.Automation.TogglePattern]::Pattern,
      [ref]$pattern
  )) {
    throw "element '$($Element.Current.Name)' does not expose TogglePattern"
  }
  $pattern.Toggle()
}

function Has-AgentTaskboard-StartupEntry {
  $path = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
  if (-not (Test-Path $path)) { return $false }
  $properties = (Get-ItemProperty $path).PSObject.Properties
  return [bool]($properties | Where-Object {
    $_.Name -notmatch '^PS' -and [string]$_.Value -match 'agent-taskboard\.exe'
  })
}

$process = Get-Process -Id $ProcessId
Wait-Until {
  $process.Refresh()
  $process.MainWindowHandle -ne [IntPtr]::Zero -and
    [AgentTaskboardNativeUi]::IsWindowVisible($process.MainWindowHandle)
} "Agent Taskboard did not expose a visible native window"
Save-Screen "01-launched.png"

$settings = Find-Visible-Element '^Settings$'
if (-not $settings) { throw "Settings button was not exposed to UI Automation" }
Invoke-Element $settings
Wait-Until { (Find-Visible-Element '^Start at login$') -ne $null } "Start at login setting did not appear"
$startAtLogin = Find-Visible-Element '^Start at login$'
Toggle-Element $startAtLogin
Wait-Until { Has-AgentTaskboard-StartupEntry } "enabling Start at login did not create a real HKCU Run entry"
Save-Screen "01a-start-at-login-enabled.png"
$startAtLogin = Find-Visible-Element '^Start at login$'
if (-not $startAtLogin) { throw "Start at login setting disappeared after enabling" }
Toggle-Element $startAtLogin
Wait-Until { -not (Has-AgentTaskboard-StartupEntry) } "disabling Start at login did not remove the real HKCU Run entry"
Save-Screen "01b-start-at-login-disabled.png"
$process.Refresh()
$windowBounds = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle).Current.BoundingRectangle
[AgentTaskboardNativeUi]::Click([int]($windowBounds.Left + 20), [int]($windowBounds.Top + 140), $false)
Wait-Until { (Find-Visible-Element '^Start at login$') -eq $null } "Settings overlay did not close"

$process.Refresh()
$windowElement = [System.Windows.Automation.AutomationElement]::FromHandle($process.MainWindowHandle)
$windowPattern = $null
if ($windowElement.TryGetCurrentPattern(
    [System.Windows.Automation.WindowPattern]::Pattern,
    [ref]$windowPattern
)) {
  $windowPattern.Close()
} elseif (-not [AgentTaskboardNativeUi]::PostMessage($process.MainWindowHandle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
  throw "the native close action could not be sent to Agent Taskboard"
}
Wait-Until {
  $process.Refresh()
  -not $process.HasExited -and
    ($process.MainWindowHandle -eq [IntPtr]::Zero -or
      -not [AgentTaskboardNativeUi]::IsWindowVisible($process.MainWindowHandle))
} "closing the window did not hide it while retaining the Host"

$stillReady = Invoke-WebRequest -UseBasicParsing http://127.0.0.1:10529/ -TimeoutSec 3
if ($stillReady.StatusCode -ne 200) { throw "Host stopped after closing the window" }
Save-Screen "02-window-hidden-host-alive.png"

$tray = Find-Tray-Element
if (-not $tray) {
  $overflow = Find-Visible-Element 'Show hidden icons|Notification Chevron|显示隐藏的图标'
  if ($overflow) {
    Click-Element $overflow
    Start-Sleep -Seconds 1
    $tray = Find-Tray-Element
  }
}
if (-not $tray) {
  $names = (All-Desktop-Elements | ForEach-Object {
    try { $_.Current.Name } catch { "" }
  } | Where-Object { $_ -match 'Agent|Taskboard|notification|icon' } | Sort-Object -Unique) -join '; '
  throw "Agent Taskboard tray icon was not exposed to UI Automation. Nearby names: $names"
}

Click-Element $tray
Wait-Until {
  $process.Refresh()
  $process.MainWindowHandle -ne [IntPtr]::Zero -and
    [AgentTaskboardNativeUi]::IsWindowVisible($process.MainWindowHandle)
} "clicking the tray icon did not reopen the window"
Save-Screen "03-tray-reopened-window.png"

$tray = Find-Tray-Element
if (-not $tray) { throw "tray icon disappeared before Quit Host verification" }
Click-Element $tray $true
Wait-Until { (Find-Visible-Element '^(Quit Host|退出 Host)$') -ne $null } "Quit Host tray menu item did not appear" 20
Save-Screen "04-tray-quit-menu.png"
$quit = Find-Visible-Element '^(Quit Host|退出 Host)$'
Click-Element $quit
Wait-Until {
  $process.Refresh()
  $process.HasExited
} "Quit Host did not exit the process"

try {
  Invoke-WebRequest -UseBasicParsing http://127.0.0.1:10529/ -TimeoutSec 2 | Out-Null
  throw "10529 remained available after Quit Host"
} catch {
  if ($_.Exception.Message -eq "10529 remained available after Quit Host") { throw }
}

Write-Host "Windows native residency smoke passed: close retained Host, tray reopened, Quit Host exited."

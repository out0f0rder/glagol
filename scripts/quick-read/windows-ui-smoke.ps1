# Public WPF fixture only: never prints extracted text or writes the clipboard.
# Click its text field when prompted. The EXE runs in its hidden text-source mode.
[CmdletBinding()]
param(
    [string]$ExePath = (Join-Path $PSScriptRoot '..\..\src-tauri\target\release\glagol.exe'),
    [ValidateRange(1, 600)][int]$FocusTimeoutSeconds = 120,
    [ValidateSet('ru-RU', 'en-US')][string]$OcrLanguage = 'ru-RU',
    [switch]$KeepOpen
)
$ErrorActionPreference='Stop'
if(-not (Test-Path -LiteralPath $ExePath -PathType Leaf)){throw 'Build the Glagol EXE before running this fixture'}
$failures=New-Object 'System.Collections.Generic.List[string]'
$incomplete=$false
Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
[StructLayout(LayoutKind.Sequential)] public struct QrFixturePoint { public int X; public int Y; }
public static class QrFixtureNative {
 [DllImport("user32.dll")] public static extern bool GetPhysicalCursorPos(out QrFixturePoint point);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
 [DllImport("user32.dll")] public static extern bool SetPhysicalCursorPos(int x,int y);
 [DllImport("user32.dll")] public static extern uint GetClipboardSequenceNumber();
}
'@
$originalWindow=[QrFixtureNative]::GetForegroundWindow()
$originalCursor=New-Object QrFixturePoint
if(-not [QrFixtureNative]::GetPhysicalCursorPos([ref]$originalCursor)){throw 'Physical cursor is unavailable'}
$clipboardSequence=[QrFixtureNative]::GetClipboardSequenceNumber()
$exe=(Resolve-Path -LiteralPath $ExePath).Path
$window=New-Object System.Windows.Window
$window.Title='Glagol Quick Read integration fixture'
$window.Width=600; $window.Height=270; $window.Left=80; $window.Top=80; $window.Topmost=$true
$panel=New-Object System.Windows.Controls.StackPanel
$run=New-Object System.Windows.Controls.Button
$run.Content='Запустить проверку UIA / OCR';$run.Height=34
$script:fixtureReady=$false
$run.Add_Click({$script:fixtureReady=$true})
$panel.Children.Add($run)|Out-Null
$box=New-Object System.Windows.Controls.TextBox
$box.FontSize=24; $box.Height=120; $box.TextWrapping='Wrap'; $box.AcceptsReturn=$true
$box.Text="Это проверка быстрого чтения.`r`nQuick Read 123."
$password=New-Object System.Windows.Controls.PasswordBox
$password.Password='public-fixture-secret'; $password.Height=40
$panel.Children.Add($box)|Out-Null; $panel.Children.Add($password)|Out-Null
$window.Content=$panel
function Pump { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 20 }
function Extract([string]$source,[string[]]$extra=@()) {
 if ([QrFixtureNative]::GetForegroundWindow() -ne $script:hwnd) {throw 'Fixture lost foreground; extraction aborted'}
 $info=New-Object System.Diagnostics.ProcessStartInfo
 $info.FileName=$script:exe; $info.Arguments=(@('--glagol-text-source',$source,"$PID")+$extra)-join ' '
 $info.StandardOutputEncoding=New-Object Text.UTF8Encoding($false); $info.UseShellExecute=$false; $info.CreateNoWindow=$true; $info.RedirectStandardOutput=$true; $info.RedirectStandardError=$true
 $process=[System.Diagnostics.Process]::Start($info)
 $reading=$process.StandardOutput.ReadToEndAsync()
 $watch=[Diagnostics.Stopwatch]::StartNew()
 while (-not $process.HasExited -and $watch.ElapsedMilliseconds -lt 18000) {Pump}
 if (-not $process.HasExited) {$process.Kill();throw 'Fixture helper timeout'}
 $json=$reading.GetAwaiter().GetResult()
 $process.Dispose()
 if ([QrFixtureNative]::GetForegroundWindow() -ne $script:hwnd) {throw 'Helper changed foreground or fixture lost focus'}
 return ($json|ConvertFrom-Json)
}
try {
 $window.Show();$window.Activate()|Out-Null;$window.UpdateLayout()
 $hwnd=(New-Object System.Windows.Interop.WindowInteropHelper($window)).Handle
 $box.Focus()|Out-Null;$box.SelectAll()
 1..12|ForEach-Object {Pump}
 Write-Output 'Fixture ready. Click its Run UIA / OCR button to start the checks.'
 $foregroundWait=[Diagnostics.Stopwatch]::StartNew()
 while (([QrFixtureNative]::GetForegroundWindow() -ne $hwnd -or -not $script:fixtureReady) -and $foregroundWait.ElapsedMilliseconds -lt ($FocusTimeoutSeconds*1000)) {Pump}
 if ([QrFixtureNative]::GetForegroundWindow() -ne $hwnd -or -not $script:fixtureReady) {throw 'Interactive fixture was not started by the user'}
 $run.IsEnabled=$false;1..6|ForEach-Object {Pump};$box.Focus()|Out-Null;$box.SelectAll();1..6|ForEach-Object {Pump}
 $selection=Extract 'selection'
 if($selection.Err){Write-Output ('Selection helper error code: '+$selection.Err)}else{Write-Output ('Selection lengths: actual='+$selection.Ok.Length+' expected='+$box.Text.Length)}
 if (($selection.Ok -replace '\s+',' ').Trim() -ne ($box.Text -replace '\s+',' ').Trim()) {throw 'UIA selected text did not match the public fixture'}
 Write-Output 'UIA selection + unchanged foreground: PASS'
 $point=$box.PointToScreen((New-Object System.Windows.Point(30,18)))
 [QrFixtureNative]::SetPhysicalCursorPos([int]$point.X,[int]$point.Y)|Out-Null
 $pointer=Extract 'pointer'
 if($pointer.Err){Write-Output ('UIA pointer error: '+$pointer.Err)}else{Write-Output ('UIA pointer output length: '+$pointer.Ok.Length)}
 if ($pointer.Ok -notmatch 'проверка') {$failures.Add('UIA pointer mismatch');Write-Output 'UIA pointer fixture text: FAIL'}else{Write-Output 'UIA pointer without click + unchanged foreground: PASS'}
 $password.Focus()|Out-Null; 1..4|ForEach-Object {Pump}
 $protected=Extract 'selection'
 if ($protected.Err -ne 'protected') {throw 'Password fixture was not rejected'}
 Write-Output 'UIA password field rejection: PASS'
 if ([QrFixtureNative]::GetClipboardSequenceNumber() -ne $clipboardSequence) {throw 'Clipboard sequence changed during fixture; preservation result is inconclusive'}
 Write-Output 'Clipboard sequence unchanged during UIA reads: PASS'
 $box.Focus()|Out-Null;1..4|ForEach-Object {Pump}
 $start=$box.PointToScreen((New-Object System.Windows.Point(0,0)))
 $end=$box.PointToScreen((New-Object System.Windows.Point($box.ActualWidth,$box.ActualHeight)))
 $coordinates=@([int]$start.X,[int]$start.Y,[int]($end.X-$start.X),[int]($end.Y-$start.Y),$OcrLanguage)|ForEach-Object {"$_"}
 $ocr=Extract 'ocr' $coordinates
 if ($ocr.Err -eq 'ocr_language') {$incomplete=$true;Write-Output 'Native OCR: NOT_RUN selected Windows language component absent'}
 elseif ($ocr.Err) {throw 'Native OCR failed on the public fixture'}
 elseif ($ocr.Ok -notmatch $(if($OcrLanguage -eq 'ru-RU'){'проверка'}else{'Quick'})) {throw 'Native OCR did not recognize the public fixture'}
 else {Write-Output 'Native OCR of public fixture: PASS'}
 Write-Output 'Automated fixture checks complete; close this window after the manual hotkey checks.'
 $box.Focus()|Out-Null;$box.SelectAll()
 $manualWait=[Diagnostics.Stopwatch]::StartNew()
 if($KeepOpen){while($window.IsVisible -and $manualWait.ElapsedMilliseconds -lt 600000){Pump}}
 if($failures.Count -gt 0){throw 'The public fixture checks failed'}
} finally {
 $window.Close()
 [QrFixtureNative]::SetPhysicalCursorPos($originalCursor.X,$originalCursor.Y)|Out-Null
 [QrFixtureNative]::SetForegroundWindow($originalWindow)|Out-Null
}

if($incomplete){exit 2}

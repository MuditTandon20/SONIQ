# Runs the production native field checks against dedicated test controls.
# No microphone, ASR, clipboard access or user document inspection.
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$probePath = Join-Path $projectRoot 'src-tauri/target/debug/examples/insertion_probe.exe'
if (-not (Test-Path -LiteralPath $probePath)) { throw 'Build first: cargo build --locked --manifest-path src-tauri/Cargo.toml --example insertion_probe' }
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class InsertionTestWindow {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr window, uint flags);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr window, System.Text.StringBuilder text, int count);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint first, uint second, bool attach);
    public static void Focus(IntPtr window) {
        uint current = GetCurrentThreadId();
        uint process;
        uint foreground = GetWindowThreadProcessId(GetForegroundWindow(), out process);
        bool attached = current != foreground && AttachThreadInput(current, foreground, true);
        try { ShowWindow(window, 9); SetForegroundWindow(window); }
        finally { if (attached) AttachThreadInput(current, foreground, false); }
    }
    public static string Describe(IntPtr window) {
        uint process; GetWindowThreadProcessId(window, out process);
        var text = new System.Text.StringBuilder(256); GetClassName(window, text, 256);
        return "pid=" + process + "; class=" + text + "; root=" + GetAncestor(window, 2);
    }
}
'@
$runDirectory = Join-Path $projectRoot ('artifacts/insertion-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
$html = Join-Path $runDirectory 'fields.html'
$fixtureTitle = 'FlowForge insertion ' + [System.IO.Path]::GetFileName($runDirectory)
@"
<!doctype html><html lang="en"><meta charset="utf-8">
<title>$fixtureTitle</title>
<h1>Fixed test text only. No microphone is active.</h1>
<label>First acceptance field<textarea id="first" aria-label="First acceptance field"></textarea></label>
<label>Second acceptance field<textarea id="second" aria-label="Second acceptance field"></textarea></label>
<label>Password acceptance field<input id="password" type="password" aria-label="Password acceptance field" autocomplete="off"></label>
<label>Read-only acceptance field<input id="readonly" readonly aria-label="Read-only acceptance field"></label>
</html>
"@ | Set-Content -LiteralPath $html -Encoding UTF8
$edgePath = Join-Path ${env:ProgramFiles(x86)} 'Microsoft/Edge/Application/msedge.exe'
if (-not (Test-Path -LiteralPath $edgePath)) { throw 'Microsoft Edge is required for these browser acceptance checks.' }
$edge = Start-Process -FilePath $edgePath -ArgumentList @('--force-renderer-accessibility', '--no-first-run', '--disable-extensions', ('--user-data-dir="' + (Join-Path $runDirectory 'edge-profile') + '"'), ('--app="' + ([Uri]$html).AbsoluteUri + '"')) -PassThru
$probe = $null
$notepadRoot = $null
try {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $probePath
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $probe = [System.Diagnostics.Process]::Start($info)
    function Probe($command) {
        $probe.StandardInput.WriteLine($command)
        $pending = $probe.StandardOutput.ReadLineAsync()
        $timeout = if ($command -eq 'long') { 60000 } else { 10000 }
        if (-not $pending.Wait($timeout)) { throw 'Native insertion probe timed out.' }
        $line = $pending.Result
        if (-not $line) { throw 'Native insertion probe exited without a result.' }
        $line | ConvertFrom-Json
    }
    function Find-Named($root, $name) {
        $condition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, $name)
        @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition) | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Edit }) | Select-Object -First 1
    }
    function Focus-Control($window, $control) {
        # Test-only focus control; production insertion never restores focus.
        [InsertionTestWindow]::Focus($window.Current.NativeWindowHandle)
        Start-Sleep -Milliseconds 300
        $control.SetFocus()
        Start-Sleep -Milliseconds 300
        if (-not $control.Current.HasKeyboardFocus -or [InsertionTestWindow]::GetAncestor([InsertionTestWindow]::GetForegroundWindow(), 2) -ne [IntPtr]$window.Current.NativeWindowHandle) { throw ('Test field did not receive foreground keyboard focus: ' + $control.Current.Name + '; window=' + $window.Current.NativeWindowHandle + '; foreground ' + [InsertionTestWindow]::Describe([InsertionTestWindow]::GetForegroundWindow())) }
    }
    function Field-Text($control) {
        # Call only for this script's empty/fixed-text non-password fixtures.
        if ($control.Current.IsPassword) { throw 'Password text must never be inspected.' }
        $pattern = $null
        if ($control.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)) { return $pattern.Current.Value }
        $control.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern).DocumentRange.GetText(-1).TrimEnd([char[]]"`r`n")
    }
    function Reset-First {
        $first.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue('')
        Focus-Control $browser $first
        $captured = Probe 'capture'
        if (-not $captured.ok) { throw ('Reset browser target rejected: ' + $captured.error) }
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(25)
    do {
        Start-Sleep -Milliseconds 250
        $desktop = [System.Windows.Automation.AutomationElement]::RootElement
        $condition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, $fixtureTitle)
        $browser = $desktop.FindFirst([System.Windows.Automation.TreeScope]::Children, $condition)
        $first = if ($browser) { Find-Named $browser 'First acceptance field' } else { $null }
    } while (-not $first -and [DateTime]::UtcNow -lt $deadline)
    if (-not $first) { throw 'Dedicated browser fixture did not become accessible.' }
    $second = Find-Named $browser 'Second acceptance field'
    $password = Find-Named $browser 'Password acceptance field'
    $readOnly = Find-Named $browser 'Read-only acceptance field'
    Focus-Control $browser $first
    $capture = Probe 'capture'
    if (-not $capture.ok) { throw ('Browser target rejected: ' + $capture.error) }
    $insert = Probe 'insert'
    if (-not $insert.ok) { throw ('Browser insertion failed: ' + $insert.error) }
    Start-Sleep -Milliseconds 200
    if ((Field-Text $first) -cne 'FlowForge acceptance 123.') { throw 'Browser did not receive the complete fixed test text.' }
    Reset-First
    $unicodeResult = Probe 'unicode'
    if (-not $unicodeResult.ok) { throw ('Browser Unicode insertion failed: ' + $unicodeResult.error) }
    Start-Sleep -Milliseconds 300
    # UTF-8 fixture bytes also work when Windows PowerShell reads this script
    # using its legacy default encoding.
    $unicodeFixture = [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('4KSo4KSu4KS44KWN4KSk4KWHIGNhZsOpIPCfmIAu'))
    if ((Field-Text $first) -cne $unicodeFixture) {
        $actualUnits = ((Field-Text $first).ToCharArray() | ForEach-Object { [int]$_ }) -join ','
        throw ('Browser Unicode fixture mismatch; UTF-16 units: ' + $actualUnits)
    }
    Reset-First
    $longBrowserResult = Probe 'long'
    Start-Sleep -Milliseconds 300
    if (-not $longBrowserResult.ok -or (Field-Text $first) -cne ('FlowForge acceptance 123. ' * 30)) { throw ('Browser long-text insertion failed: ' + $longBrowserResult.error) }
    Reset-First
    $boundaryResult = Probe 'boundary'
    Start-Sleep -Milliseconds 300
    if (-not $boundaryResult.ok -or (Field-Text $first) -cne (('FlowForge acceptance 123. ' * 5) + 'End')) { throw 'Browser boundary insertion failed.' }
    Reset-First
    Focus-Control $browser $first
    if (-not (Probe 'capture').ok) { throw 'Cannot capture first browser target for focus-change check.' }
    Focus-Control $browser $second
    $changed = Probe 'insert'
    if ($changed.ok -or (Field-Text $first) -ne '' -or (Field-Text $second) -ne '') { throw 'Focus-change refusal failed.' }
    Focus-Control $browser $password
    if ((Probe 'capture').ok) { throw 'Password field was accepted.' }
    if ((Probe 'insert').ok) { throw 'Rejected target was reused for insertion.' }
    Focus-Control $browser $readOnly
    if ((Probe 'capture').ok) { throw 'Read-only field was accepted.' }
    Focus-Control $browser $first
    if (-not (Probe 'capture').ok) { throw 'Cannot capture browser for cross-window check.' }
    # Use a uniquely named, newly created file; never read an existing document.
    $notePath = Join-Path $runDirectory ('FlowForge-acceptance-' + [Guid]::NewGuid().ToString('N') + '.txt')
    [System.IO.File]::WriteAllText($notePath, '')
    # This interactive target must be visible for UI Automation and real focus.
    Start-Process notepad.exe -ArgumentList ('"' + $notePath + '"') | Out-Null
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    do {
        Start-Sleep -Milliseconds 250
        $windows = $desktop.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
        $notepadRoot = @($windows | Where-Object { $_.Current.Name -like ('*' + [System.IO.Path]::GetFileNameWithoutExtension($notePath) + '*') }) | Select-Object -First 1
    } while (-not $notepadRoot -and [DateTime]::UtcNow -lt $deadline)
    if (-not $notepadRoot) { throw 'Dedicated Notepad fixture did not open.' }
    $condition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Document)
    $documents = @($notepadRoot.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition) | Where-Object { -not $_.Current.IsOffscreen })
    if ($documents.Count -ne 1) { throw 'Expected one dedicated Notepad document.' }
    $document = $documents[0]
    Focus-Control $notepadRoot $document
    # The previous browser target must also be refused across windows.
    if ((Probe 'insert').ok) { throw 'A rejected browser target was reused in Notepad.' }
    $capture = Probe 'capture'
    if (-not $capture.ok) { throw ('Notepad target rejected: ' + $capture.error) }
    $insert = Probe 'insert'
    if (-not $insert.ok) { throw ('Notepad insertion failed: ' + $insert.error) }
    Start-Sleep -Milliseconds 200
    if ((Field-Text $document) -cne 'FlowForge acceptance 123.') { throw 'Notepad did not receive the complete fixed test text.' }
    [System.Windows.Forms.SendKeys]::SendWait('^a{BACKSPACE}')
    Focus-Control $notepadRoot $document
    if (-not (Probe 'capture').ok) { throw 'Cannot recapture Notepad for Unicode test.' }
    $unicodeResult = Probe 'unicode'
    Start-Sleep -Milliseconds 300
    if (-not $unicodeResult.ok -or (Field-Text $document) -cne $unicodeFixture) { throw ('Notepad Unicode insertion failed: ' + $unicodeResult.error) }
    [System.Windows.Forms.SendKeys]::SendWait('^a{BACKSPACE}')
    Focus-Control $notepadRoot $document
    if (-not (Probe 'capture').ok) { throw 'Cannot recapture Notepad for long-text test.' }
    $longResult = Probe 'long'
    Start-Sleep -Milliseconds 300
    $longActual = Field-Text $document
    if (-not $longResult.ok -or $longActual -cne ('FlowForge acceptance 123. ' * 30)) {
        throw ('Notepad long-text insertion failed: ' + $longResult.error + '; received length=' + $longActual.Length + '; expected=780')
    }
    [System.Windows.Forms.SendKeys]::SendWait('^a{BACKSPACE}')
    Focus-Control $notepadRoot $document
    if (-not (Probe 'capture').ok) { throw 'Cannot capture Notepad for cancellation test.' }
    $cancelled = Probe 'cancel_after_first'
    Start-Sleep -Milliseconds 200
    if ($cancelled.ok -or (Field-Text $document) -cne 'F' -or $cancelled.error -notlike '*partially inserted*') { throw 'Cancellation did not stop native insertion after its first character.' }
    [System.Windows.Forms.SendKeys]::SendWait('^a{BACKSPACE}')
    Focus-Control $notepadRoot $document
    if (-not (Probe 'capture').ok) { throw 'Cannot capture Notepad for interrupted insertion.' }
    $probe.StandardInput.WriteLine('long')
    $pendingInsertion = $probe.StandardOutput.ReadLineAsync()
    Start-Sleep -Milliseconds 150
    Focus-Control $browser $second
    if (-not $pendingInsertion.Wait(10000)) { throw 'Insertion did not stop after a focus change.' }
    $interrupted = $pendingInsertion.Result | ConvertFrom-Json
    if ($interrupted.ok -or (Field-Text $second) -ne '' -or $interrupted.error -notlike '*partially inserted*') { throw 'Mid-insertion focus refusal failed.' }
    Focus-Control $notepadRoot $document
    # Save only this generated fixture, then close only its tab.
    [System.Windows.Forms.SendKeys]::SendWait('^s')
    Start-Sleep -Milliseconds 200
    [System.Windows.Forms.SendKeys]::SendWait('^w')
    [PSCustomObject]@{BrowserInsertion=$true;BrowserLongText=$true;Browser133Characters=$true;UnicodeInsertion=$true;SameWindowFocusChangeRefused=$true;MidInsertionFocusChangeRefused=$true;InsertionCancellation=$true;PasswordRefused=$true;ReadOnlyRefused=$true;NotepadInsertion=$true;NotepadLongText=$true;MicrophoneActivated=$false;TestDirectory=$runDirectory} | ConvertTo-Json -Compress
} finally {
    if ($probe -and -not $probe.HasExited) { $probe.StandardInput.WriteLine('quit'); if (-not $probe.WaitForExit(3000)) { $probe.Kill() } }
    # Only the isolated browser profile opened above belongs to this check.
    $profileArgument = '--user-data-dir="' + (Join-Path $runDirectory 'edge-profile') + '"'
    Get-CimInstance Win32_Process -Filter "name='msedge.exe'" | Where-Object { $_.CommandLine -and $_.CommandLine.Contains($profileArgument) } | ForEach-Object { Stop-Process -Id $_.ProcessId -ErrorAction SilentlyContinue }
}

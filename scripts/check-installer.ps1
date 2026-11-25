# Fresh-runner install and frozen-worker checks. No microphone or model download.
param([Parameter(Mandatory=$true)][string]$InstallerPath)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$installer = (Resolve-Path -LiteralPath $InstallerPath).Path
$registryPath = 'HKCU:/Software/Microsoft/Windows/CurrentVersion/Uninstall/SONIQ'
if (Test-Path -LiteralPath $registryPath) { throw 'Preserve the existing installation; uninstall it separately before this isolated check.' }
if (Get-Process soniq,soniq-worker -ErrorAction SilentlyContinue) { throw 'Close SONIQ before this check.' }
$testRoot = Join-Path $env:TEMP ('SONIQ-installer-' + [Guid]::NewGuid().ToString('N'))
$installRoot = Join-Path $testRoot 'installed'
New-Item -ItemType Directory -Path $testRoot | Out-Null
$worker = $null
$installed = $false
try {
    $process = Start-Process -FilePath $installer -ArgumentList "/S /NS /D=$installRoot" -PassThru -WindowStyle Hidden
    if (-not $process.WaitForExit(60000)) { throw 'Installer timed out.' }
    if ($process.ExitCode -ne 0) { throw "Installer exit code $($process.ExitCode)." }
    $installed = $true
    foreach ($relative in @('soniq.exe','worker/soniq-worker.exe','LICENSE','THIRD_PARTY_NOTICES.md','licenses/CTranslate2-4.9.0-LICENSE.txt','licenses/tqdm-4.70.1-LICENCE.txt','licenses/MPL-2.0.txt','uninstall.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $installRoot $relative) -PathType Leaf)) { throw "Missing resource $relative." }
    }
    foreach ($source in Get-ChildItem -LiteralPath (Join-Path $projectRoot 'docs/licenses') -File) {
        if ((Get-FileHash -LiteralPath $source.FullName).Hash -ne (Get-FileHash -LiteralPath (Join-Path $installRoot ('licenses/' + $source.Name))).Hash) { throw 'Supplementary license hash mismatch.' }
    }
    foreach ($relative in @('LICENSE','THIRD_PARTY_NOTICES.md')) {
        $source = if ($relative -eq 'LICENSE') { Join-Path $projectRoot 'LICENSE' } else { Join-Path $projectRoot 'docs/THIRD_PARTY_NOTICES.md' }
        if ((Get-FileHash -LiteralPath $source).Hash -ne (Get-FileHash -LiteralPath (Join-Path $installRoot $relative)).Hash) { throw "Notice hash mismatch: $relative." }
    }
    Write-Output 'Installed executable, worker and notice hashes verified.'
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = Join-Path $installRoot 'worker/soniq-worker.exe'
    $info.WorkingDirectory = $testRoot
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    # Avoid the checkout, external Python, and any pre-existing model cache.
    $info.EnvironmentVariables['PATH'] = "$env:SystemRoot\System32;$env:SystemRoot"
    $info.EnvironmentVariables.Remove('PYTHONHOME')
    $info.EnvironmentVariables.Remove('PYTHONPATH')
    $info.EnvironmentVariables['HF_HOME'] = Join-Path $testRoot 'empty-model-cache'
    $info.EnvironmentVariables['HF_HUB_OFFLINE'] = '1'
    $worker = [System.Diagnostics.Process]::Start($info)
    $stderr = $worker.StandardError.ReadToEndAsync()
    $worker.StandardInput.WriteLine('{"command":"load","id":1,"model":"tiny","download":false}')
    $response = $worker.StandardOutput.ReadLineAsync()
    if (-not $response.Wait(60000) -or -not $response.Result) { throw 'Frozen worker did not respond.' }
    $message = $response.Result | ConvertFrom-Json
    if ($message.event -ne 'error' -or $message.id -ne 1 -or $message.message -notlike '*Cannot load model*tiny*Use --download*') { throw 'Expected the explicit offline missing-model response from the installed speech engine.' }
    Write-Output 'Installed frozen worker handled the offline missing-model request without external Python.'
    $worker.StandardInput.WriteLine('{"command":"shutdown"}')
    if (-not $worker.WaitForExit(10000) -or $worker.ExitCode -ne 0) { throw 'Frozen worker did not shut down successfully.' }
} finally {
    if ($worker -and -not $worker.HasExited) { $worker.Kill(); $worker.WaitForExit() }
    if ($installed) {
        $registered = Get-ItemProperty -LiteralPath $registryPath
        if ($registered.UninstallString -ne ('"' + (Join-Path $installRoot 'uninstall.exe') + '"')) { throw 'Uninstall registration differs from this test installation.' }
        $uninstall = Start-Process -FilePath (Join-Path $installRoot 'uninstall.exe') -ArgumentList '/S' -PassThru -WindowStyle Hidden
        if (-not $uninstall.WaitForExit(30000)) { throw 'Uninstaller launcher timed out.' }
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        while (((Test-Path -LiteralPath $installRoot) -or (Test-Path -LiteralPath $registryPath)) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 250 }
        if ((Test-Path -LiteralPath $installRoot) -or (Test-Path -LiteralPath $registryPath)) { throw ("Uninstall incomplete: directory=" + (Test-Path -LiteralPath $installRoot) + "; registration=" + (Test-Path -LiteralPath $registryPath) + "; launcher exit=" + $uninstall.ExitCode) }
    }
}
[PSCustomObject]@{Install=$true;NoticeHashes=$true;FrozenWorkerWithoutExternalPython=$true;OfflineMissingModel=$true;Uninstall=$true;MicrophoneActivated=$false;ModelDownloaded=$false;InstallerSHA256=(Get-FileHash -LiteralPath $installer).Hash;SignatureStatus=(Get-AuthenticodeSignature -LiteralPath $installer).Status.ToString()} | ConvertTo-Json -Compress

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
$pythonPath = Join-Path $projectRoot '.venv\Scripts\python.exe'
if (-not (Test-Path -LiteralPath $pythonPath)) { throw 'Create .venv and install .[package] first. See README.' }
& $pythonPath -m PyInstaller --noconfirm --clean --onedir --name soniq-worker --distpath src-tauri/resources --workpath build/pyinstaller --specpath build --collect-all faster_whisper --collect-all ctranslate2 --collect-all tokenizers --collect-all av --collect-all onnxruntime --collect-all huggingface_hub scripts/worker_entry.py
if ($LASTEXITCODE -ne 0) { throw 'Worker packaging failed.' }
Write-Output "Packaged speech engine: $projectRoot\src-tauri\resources\soniq-worker"
exit 0

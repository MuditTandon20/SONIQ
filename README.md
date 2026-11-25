# SONIQ

Local, Windows-first voice dictation with a Tauri desktop interface and a reusable Python speech engine. Audio recognition stays on your device. The CLI remains available for recording, evaluation and experiments.

Source repository: [MuditTandon20/SONIQ](https://github.com/MuditTandon20/SONIQ).

Application code is MIT licensed; dependencies, evaluation audio and model weights retain their own licenses.

## Desktop (Windows PowerShell)

After the Python setup below, install the desktop dependencies and run:

```powershell
npm ci
npm run desktop
```

Development requires Node 22.12+, Rust stable with the MSVC target, Visual Studio C++ Build Tools and Windows WebView2. The installed application bundles Python and its inference libraries; development uses `.venv\Scripts\python.exe`.

1. Wait for the local engine to become ready. If weights are missing, select **Download selected model**; downloading is explicit.
   Check **Preferences → Microphone** and explicitly select the input you intend to speak into. **Windows default input** may still be the laptop microphone even when a headset is connected.
2. Click the microphone to start/stop a preview recording, or hold **Ctrl+Space** while another text field is focused. Release it to transcribe. Preferences can change the shortcut or preferred microphone. The recording status names the input actually in use. If a microphone disconnects during capture, the recording fails and is discarded; the next explicit recording checks devices again.
3. A separate indicator stays visible without taking focus. Each recording is capped at 120 seconds. Cancel using the app/indicator button, or Escape while the main app is focused.
4. Review the transcript and switch to **Original** to recover the ASR result. **Copy** explicitly writes the selected transcript to the Windows clipboard.
5. Optional automatic insertion verifies the original window, process, UI Automation field identity and password status. It never restores focus or reads target text. A changed or unsupported target keeps the transcript for manual copying.

Automatic insertion is off by default. Windows offers no atomic focus-check-and-type operation: another app can change focus in the brief interval before Unicode input delivery. Use manual copy for sensitive applications. Multiline/control-character transcripts also require manual copying to avoid sending form-submission keys. Elevated apps, protected fields, custom editors and providers without reliable accessibility metadata may reject insertion. Successful input delivery cannot prove the application accepted every character. Notepad and browser-field acceptance checks must be reported separately from native unit tests.

Preferences offer English/Hindi/automatic language detection, tiny/base/small models, int8/float32 CPU execution, terminology hints, optional Silero silence filtering, and opt-in cleanup. Verbatim is the default. Cleanup only adjusts whitespace, optional “um”/“uh” removal, and sentence paragraph breaks; digit-bearing and recognizable technical text stays verbatim. Names and facts still require review. No LLM is enabled without a fixed-set quality comparison demonstrating a useful improvement.

Experimental live preview refreshes a rolling 12-second window at most once every four seconds, with one request in flight. It is provisional, may repeat/change text, and is never inserted. Final ASR uses the complete bounded recording. This is windowed re-transcription, not an incremental Whisper decoder. Silence filtering does not automatically stop recording; the user controls boundaries.

Settings are validated and atomically replaced under `%APPDATA%\ai.soniq.desktop\settings.json`. Temporary desktop captures live under `%LOCALAPPDATA%\ai.soniq.desktop\captures`; normal completion/cancellation removes them, and startup removes leftover app-owned session directories after crashes. Deletion is not secure erasure. A named Windows mutex prevents multiple app instances. Closing the app exits and terminates its supervised worker. No autostart or background microphone activation is configured.

## Build an installer

Automated desktop checks on 2026-11-25 exercised the actual insertion code with dedicated Edge and Notepad fixtures, including Hindi characters, accented text, emoji, 133- and 780-character text, focus changes, password/read-only refusal and cancellation of native insertion. These checks used fixed test strings and no microphone. Modern Notepad required synchronous delivery to its verified Unicode editor window; other fields receive paced Unicode input. Each character rechecks focus, modifiers and the active session. Interrupted insertion can leave text already delivered in the target; cancellation does not undo it. Review the field before copying again.

Close the running SONIQ desktop app before packaging, and wait for packaging to finish before launching the release executable.

```powershell
.\.venv\Scripts\python.exe -m pip install -e '.[test,package]'
npm run package
```

This packages the speech worker with PyInstaller and creates a per-user NSIS installer under `src-tauri\target\release\bundle\nsis`. Model weights are downloaded separately; they are not embedded or committed. WebView2 installation may require a download if the Windows runtime is missing. Local builds are unsigned, so Windows may show an unknown-publisher warning. This is not a claim of code signing, clean-machine validation or a published GitHub release.

The installed resource directory includes `LICENSE` and `THIRD_PARTY_NOTICES.md` alongside the bundled worker. These notices do not establish a complete redistribution audit; public distribution remains a separate release gate.

The current unsigned installer has passed isolated-directory install/uninstall and native startup, single-instance and idle worker shutdown/crash checks on the development machine. Physical start/stop and indicator visibility were checked with the owner. Recognition still needs review; the limited headset and vocabulary comparisons are recorded in [CHALLENGES](docs/CHALLENGES.md). Clean-machine validation and target-app acceptance remain separate gates.

```powershell
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

The Windows GitHub Actions workflow runs Python, frontend and Rust checks; configuring a workflow is separate from observing it pass remotely.

The verified source passed [remote Windows checks, including strict Clippy](https://github.com/MuditTandon20/SONIQ/actions/runs/45586073415). The separate [Windows installer smoke workflow passed](https://github.com/MuditTandon20/SONIQ/actions/runs/45583459268): it builds and installs on a fresh hosted runner, checks the frozen worker with external Python removed from its PATH and an empty offline model cache, then uninstalls. Hosted-runner checks do not establish clean consumer-machine, microphone or speech-quality acceptance. No binary is automatically published.

To repeat insertion acceptance, leave the dedicated test windows untouched while the check runs:

```powershell
cargo build --locked --manifest-path src-tauri/Cargo.toml --example insertion_probe
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-native-insertion.ps1
```

The helper invokes the production native field code and accepts only fixed fixture cases. It opens isolated Edge and Notepad fixtures under ignored `artifacts/`, never opens the microphone, and never reads existing documents or clipboard contents.

## Setup (Windows PowerShell)

```powershell
Set-Location .\SONIQ
py -3.10 -m venv .venv
.\.venv\Scripts\python.exe -m pip install -e '.[test]'
.\.venv\Scripts\python.exe -m pytest -q
```

Python 3.10 or later is required. Activation is unnecessary. The faster-whisper dependency uses native CTranslate2 and PyAV wheels; if Windows reports a missing runtime DLL, install Microsoft's supported Visual C++ x64 Redistributable. No CUDA or system FFmpeg installation is required for this CPU baseline.

## Record and transcribe

```powershell
.\.venv\Scripts\python.exe -m soniq devices
.\.venv\Scripts\python.exe -m soniq record recordings\sample.wav --seconds 8
# Choose an input device by the index printed above, if needed:
.\.venv\Scripts\python.exe -m soniq record recordings\other.wav --seconds 8 --device 1
.\.venv\Scripts\python.exe -m soniq transcribe recordings\sample.wav --model base --language en --download
# Later runs use cached model files and default to offline loading:
.\.venv\Scripts\python.exe -m soniq transcribe recordings\sample.wav --model base --language en
```

Recording waits for Enter, displays RECORDING, then stops after the requested duration (maximum 600 seconds). Ctrl+C discards partial capture. Existing output files are never overwritten. Windows must allow microphone access for desktop apps. Device indexes can change when hardware changes. A disconnected or busy microphone returns an error; input overflow discards the recording rather than silently accepting missing samples.

`--download` permits the faster-whisper model downloader to fetch weights from identified Hugging Face repositories, such as [Systran/faster-whisper-base](https://huggingface.co/Systran/faster-whisper-base). It never uploads your audio. Downloads can be large; remove the flag once cached. A local converted model directory also works as `--model`. Use multilingual `tiny`, `base`, or `small` for Hindi; omit `--language` for detection or use `hi`. A preliminary licensed 20-clip Hindi test had high word error rates, including about 57.5% with small and VAD through the packaged worker. Hindi/Hinglish should be treated as experimental; reliable everyday quality has not been established. See CHALLENGES for the pinned subset and limitations. Review each model's license before redistribution.

Transcription accepts multiple files and loads the model once per command. The default is multilingual base, CPU, int8, beam size 5. Use `--compute-type float32` or `--beam-size 1` for measured comparisons. WAV, FLAC and other formats supported by PyAV are decoded to mono 16 kHz. Exact digital silence returns an empty transcript; ambient noise can still produce hallucinations. No VAD or cleanup model is enabled.

## Evaluate real recordings

Create a UTF-8 JSONL file containing one object per line. Paths are relative to the manifest. References must be independently checked against the audio, never copied from ASR output. Use consented or licensed audio.

```powershell
New-Item -ItemType Directory -Force reports | Out-Null
'{"audio":"../recordings/sample.wav","reference":"The words actually spoken in your recording."}' | Set-Content -Encoding UTF8 reports\evaluation.jsonl
.\.venv\Scripts\python.exe -m soniq evaluate reports\evaluation.jsonl --model tiny base --language en --download --report reports\comparison.json
.\.venv\Scripts\python.exe -m soniq evaluate reports\evaluation.jsonl --model base --compute-type float32 --language en --repeats 3
```

Replace the example reference with your real transcript. Reports contain corpus WER/CER, model startup, audio decoding and inference times, aggregate real-time factor, median latency, configuration and dependency versions. p95 is reported only for at least 20 observations. Repetitions reuse the model but include the first inference; there is no hidden warmup. One clip cannot establish general accuracy. Compare identical recordings, references, language and decoding settings. Normalization applies NFC, case folding, punctuation-to-spaces and whitespace collapse; Hindi vowel marks remain intact. Spaces count in CER. Empty references permit silence/hallucination tests; jiwer counts insertions for these cases.

JSON is printed to the terminal. Transcripts are persisted only if you explicitly redirect output or request `--report`; report paths must not exist already. Shell history and terminal scrollback can still retain text. Recordings remain at the output paths you select. The project lives in OneDrive: local files may be synced by your OS account independently of this application. Choose a non-synced location for sensitive recordings, reports, or model caches. The application has no telemetry, clipboard monitoring, web server, or transcript history.

## Verification and boundaries

See [CHALLENGES](docs/CHALLENGES.md) for actual verification results and limitations, [DECISIONS](docs/DECISIONS.md) for dependencies, [ARCHITECTURE](docs/ARCHITECTURE.md) for execution paths, and [LEARNING](docs/LEARNING.md) for concepts. Tests create real audio files but mock microphone capture and model lifecycle where hardware independence is needed. Those tests do not prove a physical microphone works. GPU execution, Hindi/Hinglish accuracy, CPU utilization and interactive capture latency remain unverified unless explicitly recorded there. Do not infer release readiness from unit tests or one familiar English sample.

## Chunk-size experiments

```powershell
.\.venv\Scripts\python.exe -m soniq experiment reports\evaluation.jsonl --model tiny --chunks 4 8 12 --language en --report reports\chunks.json
.\.venv\Scripts\python.exe -m soniq experiment reports\evaluation.jsonl --model tiny --chunks 4 8 12 --language en --vad --report reports\chunks-vad.json
```

Models must already be cached. Independent non-overlapping chunks report corpus WER/CER, per-chunk latency, RTF and Windows peak process memory. Punctuation/boundary differences may be hidden by normalized WER. This experiment does not measure live microphone-to-preview latency or stabilized streaming. Use a held-out, consented multilingual corpus before quality conclusions or fine-tuning.

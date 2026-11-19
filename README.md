# FlowForge AI

Local, Windows-first speech recognition. Milestone 1 provides a Python CLI for microphone capture, audio-file transcription, and reference-based model evaluation. Desktop shortcuts and text insertion are later work.

Application code is MIT licensed; dependencies, evaluation audio and model weights retain their own licenses.

## Setup (Windows PowerShell)

```powershell
Set-Location 'C:\Users\Mudit\OneDrive\Desktop\Python\FlowForge AI'
py -3.10 -m venv .venv
.\.venv\Scripts\python.exe -m pip install -e '.[test]'
.\.venv\Scripts\python.exe -m pytest -q
```

Python 3.10 or later is required. Activation is unnecessary. The faster-whisper dependency uses native CTranslate2 and PyAV wheels; if Windows reports a missing runtime DLL, install Microsoft's supported Visual C++ x64 Redistributable. No CUDA or system FFmpeg installation is required for this CPU baseline.

## Record and transcribe

```powershell
.\.venv\Scripts\python.exe -m flowforge devices
.\.venv\Scripts\python.exe -m flowforge record recordings\sample.wav --seconds 8
# Choose an input device by the index printed above, if needed:
.\.venv\Scripts\python.exe -m flowforge record recordings\other.wav --seconds 8 --device 1
.\.venv\Scripts\python.exe -m flowforge transcribe recordings\sample.wav --model base --language en --download
# Later runs use cached model files and default to offline loading:
.\.venv\Scripts\python.exe -m flowforge transcribe recordings\sample.wav --model base --language en
```

Recording waits for Enter, displays RECORDING, then stops after the requested duration (maximum 600 seconds). Ctrl+C discards partial capture. Existing output files are never overwritten. Windows must allow microphone access for desktop apps. Device indexes can change when hardware changes. A disconnected or busy microphone returns an error; input overflow discards the recording rather than silently accepting missing samples.

`--download` permits the faster-whisper model downloader to fetch weights from identified Hugging Face repositories, such as [Systran/faster-whisper-base](https://huggingface.co/Systran/faster-whisper-base). It never uploads your audio. Downloads can be large; remove the flag once cached. A local converted model directory also works as `--model`. Use multilingual `tiny`, `base`, or `small` for Hindi; omit `--language` for detection or use `hi`. Hindi/Hinglish quality has not been established. Review each model's license before redistribution.

Transcription accepts multiple files and loads the model once per command. The default is multilingual base, CPU, int8, beam size 5. Use `--compute-type float32` or `--beam-size 1` for measured comparisons. WAV, FLAC and other formats supported by PyAV are decoded to mono 16 kHz. Exact digital silence returns an empty transcript; ambient noise can still produce hallucinations. No VAD or cleanup model is enabled.

## Evaluate real recordings

Create a UTF-8 JSONL file containing one object per line. Paths are relative to the manifest. References must be independently checked against the audio, never copied from ASR output. Use consented or licensed audio.

```powershell
New-Item -ItemType Directory -Force reports | Out-Null
'{"audio":"../recordings/sample.wav","reference":"The words actually spoken in your recording."}' | Set-Content -Encoding UTF8 reports\evaluation.jsonl
.\.venv\Scripts\python.exe -m flowforge evaluate reports\evaluation.jsonl --model tiny base --language en --download --report reports\comparison.json
.\.venv\Scripts\python.exe -m flowforge evaluate reports\evaluation.jsonl --model base --compute-type float32 --language en --repeats 3
```

Replace the example reference with your real transcript. Reports contain corpus WER/CER, model startup, audio decoding and inference times, aggregate real-time factor, median latency, configuration and dependency versions. p95 is reported only for at least 20 observations. Repetitions reuse the model but include the first inference; there is no hidden warmup. One clip cannot establish general accuracy. Compare identical recordings, references, language and decoding settings. Normalization applies NFC, case folding, punctuation-to-spaces and whitespace collapse; Hindi vowel marks remain intact. Spaces count in CER. Empty references permit silence/hallucination tests; jiwer counts insertions for these cases.

JSON is printed to the terminal. Transcripts are persisted only if you explicitly redirect output or request `--report`; report paths must not exist already. Shell history and terminal scrollback can still retain text. Recordings remain at the output paths you select. The project lives in OneDrive: local files may be synced by your OS account independently of this application. Choose a non-synced location for sensitive recordings, reports, or model caches. The application has no telemetry, clipboard monitoring, web server, or transcript history.

## Verification and boundaries

See [CHALLENGES](docs/CHALLENGES.md) for actual verification results and limitations, [DECISIONS](docs/DECISIONS.md) for dependencies, [ARCHITECTURE](docs/ARCHITECTURE.md) for the execution path, and [LEARNING](docs/LEARNING.md) for the concepts. Tests create real audio files but mock microphone capture and model lifecycle where hardware independence is needed. Those tests do not prove a physical microphone works. GPU execution, Hindi/Hinglish evaluation, peak memory, CPU utilization, and interactive capture latency are not validated in this milestone.

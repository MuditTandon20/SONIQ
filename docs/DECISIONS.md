# Decisions

## D1 — Python CLI before desktop integration

Requirement: establish working ASR and measurable quality. Simplest solution: one Python process and CLI arguments. Rejected Tauri, servers and worker IPC until a UI needs them. Cost: blocking commands and no global shortcut. Evidence: current pipeline has no interprocess communication requirement. Reconsider after real microphone and recognition checks pass.

## D2 — faster-whisper on CPU int8

Requirement: local multilingual inference on consumer Windows hardware. Use the requested faster-whisper/CTranslate2 stack with a reusable model. CPU avoids CUDA installation. Base is the initial balanced candidate; tiny is a cheaper comparison, not an asserted accuracy winner. Cost: native wheels, model weights, and sizeable transitive dependencies (including ONNX Runtime even with VAD disabled). Evidence: detected i5-12450HX, 12 logical CPUs and about 11.7 GiB RAM; actual evaluation results are recorded separately. Reconsider the model/compute type using the same measured corpus. See [upstream usage and lazy segments](https://github.com/SYSTRAN/faster-whisper).

## D3 — sounddevice plus standard-library WAV

Requirement: enumerate and record Windows microphones. sounddevice supplies PortAudio bindings; the standard library cannot capture microphones portably. `wave`, temporary files and a bounded read loop avoid an additional file-format library and unbounded RAM buffers. Cost: PortAudio device/driver behavior and explicit timed recordings. Evidence: automated WAV and failure-path tests; physical capture still needs interactive verification. Reconsider control flow for desktop push-to-talk. [Raw stream documentation](https://python-sounddevice.readthedocs.io/en/latest/api/raw-streams.html).

## D4 — NumPy, PyAV and jiwer

Requirement: resample input, validate samples, calculate reproducible error metrics. Reuse PyAV already installed by faster-whisper; declare NumPy directly because our code uses it. jiwer provides tested edit-distance metrics rather than a custom implementation. Cost: numerical and metric packages; no pandas or experiment server. Evidence: real resampling tests and known-answer WER/CER checks. Reconsider only when dataset analysis exceeds JSON reports. [jiwer semantics](https://github.com/jitsi/jiwer).

## D5 — pytest, CLI arguments and opt-in downloads

Requirement: repeatable failure-boundary tests, minimal configuration and offline runtime. pytest is development-only. CLI arguments avoid a settings persistence layer; no settings file is created. Downloads are disabled unless requested. Cost: first-use model preparation and explicit command arguments. Evidence: offline flag propagation and invalid configuration tests. Reconsider settings persistence when the desktop UI needs saved preferences. No extra runtime abstraction is introduced beyond the reusable Transcriber.

Model resolution uses faster-whisper's existing downloader and checks for tokenizer.json before constructing WhisperModel. This prevents upstream's missing-tokenizer network fallback even for a supplied local directory. See [upstream constructor](https://github.com/SYSTRAN/faster-whisper/blob/master/faster_whisper/transcribe.py). We add no separate download client.

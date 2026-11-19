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

## D6 — Desktop stack and native audio

Requirement: responsive Windows dictation, now authorized by the owner. Use Tauri v2, React/TypeScript, plain CSS and React state; reject a hosted product or local server. CPAL captures native audio; hound writes WAV; tempfile owns sessions and atomic settings. Windows bindings provide focused-control metadata, clipboard/input, single-instance mutex and kill-on-close worker jobs. Standard Rust APIs do not cover these OS capabilities. Cost: Rust/MSVC/Node build tooling, native failure cases and WebView2. Evidence: frontend typecheck/build and Python real-ASR checks pass; native results are recorded separately. Reconsider based on measured startup, packaging or maintenance costs, not speculative abstractions.

## D7 — Supervised NDJSON worker

Requirement: keep inference off the UI thread and reuse weights. One stdin/stdout worker with temporary WAV paths is sufficient; reject HTTP/WebSocket infrastructure and multiple inference processes. Cost: session IDs, worker generations, cleanup and packaged runtime. Evidence: real source-worker and frozen-worker load/transcribe/shutdown checks pass. Reconsider binary transport only if benchmarks show path/file overhead matters.

## D8 — Conservative correction and optional VAD

Requirement: useful formatting without hidden factual rewrites. Verbatim default, recoverable raw text, optional whitespace/filler/paragraph cleanup and ASR hotword hints. Numeric/recognizable technical text bypasses cleanup. Silero is already provided by faster-whisper/ONNX Runtime, so no extra VAD dependency is added. Cost: quiet speech can be omitted; filler removal can be unsuitable for some names; hints do not guarantee spelling. Evidence: regression cases and real VAD inference. No LLM candidate has demonstrated a fixed-set quality/latency improvement, so none is enabled. Reconsider with a held-out correction corpus.

## D9 — Bounded optional preview and experiments

Requirement: preview speech before stop without an unbounded queue. Use one in-flight preview, four-second cadence and twelve-second context; final full-clip ASR remains authoritative. Cost: repeated work, unstable partials and possible final latency. Evidence: real 4/8/12-second independent-chunk comparisons with/without VAD showed more aggregate work and punctuation variation with shorter chunks on one clip. This does not prove an optimal context or multilingual quality. Keep preview off by default; reconsider stabilization/endpointing after broader measurements.

## D10 — Packaging and checks

Requirement: install without a separate Python environment and keep verification repeatable. Packaging-only PyInstaller produces an onedir resource, avoiding extraction of the inference runtime on every launch; Tauri NSIS installs per user. Windows GitHub Actions runs local checks once published. Cost: installer size, native DLL/license review, unsigned builds and clean-machine verification. Evidence: frozen worker runs real ASR; installer and remote CI results must be observed separately. No Docker, updater or telemetry is added. Reconsider signing/updates for a public stable release.

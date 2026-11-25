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

The release resource map also installs the application's `LICENSE`, `docs/THIRD_PARTY_NOTICES.md` and supplementary `docs/licenses/` next to the worker. This uses Tauri's existing resource mechanism and adds no dependency. Bundling notices is separate from completing the exact-build redistribution audit. Keep the application closed throughout packaging to avoid a live executable interfering with bundling.

## D11 - Verified native text delivery

Actual Notepad and browser fixtures corrupted or dropped text despite a successful whole-string `SendInput` return. Modern Notepad's verified Unicode `RichEditD2DPT` window now receives synchronous `WM_CHAR` messages with a one-second timeout. Other supported fields receive one Unicode scalar per `SendInput` call, with 20 ms pacing; surrogate pairs stay together. Both paths recheck the field identity, modifiers and current session before each character. Failures after a delivery attempt warn about partial insertion. Cancel cannot retract already delivered text. This uses the existing Windows crate, adds no dependency, and deliberately trades insertion speed for observed fixture correctness. It still cannot guarantee acceptance by every target application.

The optional acceptance helper compiles the production native module and accepts fixed test cases only. The PowerShell harness controls newly created test windows and inspects only their fixture contents. It never opens a microphone or uses ASR. `scripts/inventory-worker.py` uses the standard library to hash actual frozen binaries and inventory bundled license evidence; its report is not a complete redistribution audit.

## D12 - Automatic microphone fallback and two-key default shortcut

The microphone setting is now a preference, not a requirement that prevents recording after unplugging a headset. `capture::start` enumerates input devices on each explicit recording, uses the preferred name if present, and otherwise asks CPAL for the current Windows default input. The fallback is not saved over the preference, so reconnecting the headset restores it on the next recording. An empty preference follows Windows. Do not guess device priority from brand names, switch to another microphone mid-capture, or start listening to discover the loudest device. Available-but-busy or unsupported inputs still report their actual opening error. This uses existing CPAL APIs and adds no dependency.

The recording status names the actual opened device. Preferences refresh device metadata on page entry and app focus without opening a stream, and retain an unavailable preference visibly in the list. A small shared selection function allows connected/disconnected/reconnected and changed-default tests without opening real hardware. Those tests are not physical microphone acceptance.

New settings default to Ctrl+Space, satisfying the owner's two-key request. Existing custom shortcuts are preserved when reading settings; this owner's active shortcut was explicitly changed to Ctrl+Space while the app was closed, retaining every other preference. Native startup still checks whether the shortcut can be registered. No model initialization, Python CLI behavior, dependency or recognition setting changed.

## D13 - SONIQ project identity (2026-11-26)

The owner requested the SONIQ name across the current project and GitHub. App/window titles, identifier, mutex and generated executables use SONIQ/soniq; the Python distribution, import package and console command are `soniq`, and the speech worker is `soniq-worker`. Existing CLI subcommands and recognition settings remain unchanged. GitHub's repository was renamed in place rather than recreated, preserving its commits and verification links. Historical benchmark measurements/hashes remain historical evidence, not a new recognition claim for the SONIQ vocabulary term.

Only project-package metadata and entry points were reinstalled in the existing venv with `--no-deps`. The already-declared isolated setuptools build environment built the renamed wheel; application dependencies and cached speech models were retained. The frozen worker was rebuilt because its Python import namespace changed. No new dependency was introduced. The neutral microphone icon was inspected and retained because it contains no old name or lettering.

After owner authorization to discard the volatile preview, the previous app was closed normally. Saved preferences were copied atomically to the new identifier's settings directory, preserving all fields except the product-name vocabulary spelling. No recordings, transcript data, model weights or WebView history were migrated. Existing pre-rebrand settings are left intact outside the repository; no compatibility import package or old-name CLI is published in the renamed source.

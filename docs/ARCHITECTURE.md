# Architecture

`python -m flowforge` calls `cli.main`. Four small modules own current responsibilities:

- `audio.py`: enumerate microphone inputs, capture native-rate mono int16 WAV, decode files to float32 mono 16 kHz.
- `asr.py`: initialize one CPU Whisper model, transcribe multiple files, consume lazy segments before stopping the inference timer.
- `evaluation.py`: validate JSONL, normalize reference/hypothesis pairs, calculate corpus metrics and latency summaries.
- `cli.py`: arguments, explicit recording confirmation, results and final error reporting.

Capture uses blocking 50 ms reads directly into a temporary WAV. It never accumulates the recording in RAM. PortAudio resources close on success, cancellation and failure. A same-directory hard link publishes the finished WAV without overwriting another file; NTFS supports this, other filesystems may return an actionable OS error. The temporary name is removed afterward. No asynchronous infrastructure is needed for timed capture. Recording does not perform recognition concurrently.

PyAV, already required by faster-whisper, handles downmixing/resampling. File transcription decodes the entire clip in memory: use short dictation clips, not multi-hour media. It does not implement streaming or backpressure. Model startup is measured separately from decoding and inference. Inference includes feature extraction, decoding and segment iteration; reported RTF excludes model load and file decoding.

The CLI remains independent. Desktop additions are described below; there is still no database or network service.

## Desktop data path

React/TypeScript renders preferences and transcript/status inside Tauri's system WebView2. Rust owns shortcuts, CPAL capture, native insertion, clipboard, settings and worker supervision. Python owns faster-whisper, optional Silero VAD and conservative text cleanup. One worker exchanges newline-delimited JSON over stdin/stdout; stderr carries diagnostics. A Windows kill-on-close job prevents orphaned worker processes after a crash.

Window configuration disables automatic creation. Setup registers the complete runtime before creating the main and indicator WebViews, so their first IPC snapshot can read preferences. A main-window close request explicitly exits the application; otherwise the hidden indicator would keep the event loop and worker alive.

1. A button or key press begins an explicit session. Shortcut recording snapshots supported focused-field metadata, never its text.
2. A dedicated CPAL thread downmixes native-rate audio to mono int16 in a preallocated buffer capped at 120 seconds. The non-focusing indicator stays visible. Inference and disk writes happen outside the audio callback.
3. Stop/release closes capture and writes a temporary WAV. Python uses PyAV to resample it to mono float32 16 kHz, reuses the loaded model and returns original/cleaned text plus timing.
4. Rust checks worker generation and session ID, discarding late results. Optional insertion verifies window, process, field runtime ID, password/read-only/enabled state and modifiers before each Unicode scalar, and checks that the session remains current. Verified Unicode `RichEditD2DPT` windows use synchronous `WM_CHAR`; other supported fields use paced Unicode `SendInput`. Failure can leave partial target text and keeps manual copying available. Windows cannot make the focus check and delivery atomic, so insertion defaults off.
5. Normal completion/cancellation drops session files. In-flight ASR cancellation kills/reloads the worker. A named mutex prevents multiple instances; startup removes app-owned stale capture directories.

Live preview is optional: at most once every four seconds, a snapshot of the last twelve seconds goes to the same worker with beam 1. One request can be in flight. Final ASR uses the full clip with beam 5. A preview can delay final processing; partials remain provisional. This is rolling re-transcription, not cumulative stabilization, incremental decoding or automatic endpointing.

Desktop settings use validated JSON and atomic replacement. Audio briefly resides in app-local captures; deletion is not secure erasure. No transcript history, text-content scraping, local LLM, training pipeline or cloud inference exists. PyInstaller bundles the worker/runtime as a folder resource; Tauri creates a per-user NSIS installer. Model preparation remains explicit.

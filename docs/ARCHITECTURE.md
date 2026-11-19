# Milestone 1 architecture

`python -m flowforge` calls `cli.main`. Four small modules own current responsibilities:

- `audio.py`: enumerate microphone inputs, capture native-rate mono int16 WAV, decode files to float32 mono 16 kHz.
- `asr.py`: initialize one CPU Whisper model, transcribe multiple files, consume lazy segments before stopping the inference timer.
- `evaluation.py`: validate JSONL, normalize reference/hypothesis pairs, calculate corpus metrics and latency summaries.
- `cli.py`: arguments, explicit recording confirmation, results and final error reporting.

Capture uses blocking 50 ms reads directly into a temporary WAV. It never accumulates the recording in RAM. PortAudio resources close on success, cancellation and failure. A same-directory hard link publishes the finished WAV without overwriting another file; NTFS supports this, other filesystems may return an actionable OS error. The temporary name is removed afterward. No asynchronous infrastructure is needed for timed capture. Recording does not perform recognition concurrently.

PyAV, already required by faster-whisper, handles downmixing/resampling. File transcription decodes the entire clip in memory: use short dictation clips, not multi-hour media. It does not implement streaming or backpressure. Model startup is measured separately from decoding and inference. Inference includes feature extraction, decoding and segment iteration; reported RTF excludes model load and file decoding.

The only persistent data is explicitly requested audio, optional evaluation reports, and the upstream model cache. No settings file is needed yet: command arguments fully define each experiment. No UI, IPC, worker, database or network service exists.

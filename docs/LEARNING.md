# Learning from Milestone 1

## Audio before ML

A sample measures air-pressure variation at one instant. A 48 kHz microphone produces 48,000 samples per second. Our WAV stores one channel with signed 16-bit samples. `record` writes blocks to disk so recording memory stays bounded. `load_audio` decodes and resamples to the 16 kHz float32 waveform expected by Whisper. Stereo downmix and sample-rate conversion are real operations, not header changes: tests check the resulting sample count and signal amplitude.

## Recognition and model lifetime

Conceptually, Whisper converts waveform windows into log-Mel features, encodes acoustic context, then generates text tokens autoregressively. This is a simplified explanation; our application delegates the implementation to faster-whisper/CTranslate2. We do not train or reimplement the transformer. Beam search retains multiple candidate token sequences; larger beams can increase compute and do not guarantee better transcription for every clip.

`Transcriber.__init__` loads weights once. `transcribe` reuses that model. faster-whisper returns lazy segments, so timing only the call would miss much of the work. Joining all segments inside the timer measures completed inference. Model loading and file decoding are reported separately. CPU int8 uses lower precision weights/computation choices supported by CTranslate2; speed, memory and quality benefits need measurement rather than assumption.

## Accuracy and latency

Word error rate is `(substitutions + deletions + insertions) / reference words`. Character error rate uses characters instead. They can exceed 1 when there are many insertions. Corpus scoring weights examples by reference length, avoiding an unweighted average of sentence scores. Normalization is part of the experiment: stripping punctuation changes what WER measures. `normalize` preserves Unicode combining marks so Hindi text is not damaged. CER here includes normalized spaces.

Real-time factor is inference seconds divided by audio seconds. RTF below 1 means inference alone is faster than the recorded duration, not that the complete user interaction has that latency. Startup and capture can still dominate. p95 from very few samples is misleading, so evaluation requires at least 20 observations before reporting it. Exact digital silence is easy to identify; room noise is not evidence of speech and Whisper may hallucinate on it. No amplitude threshold is used to discard quiet speech.

## Experiments and interview practice

Record the same short English, Hindi and Hinglish sentences with consent and manually verify references. Compare tiny/base, int8/float32 and beam 1/5 separately. Keep the corpus fixed; record startup and inference independently. Include silence, quiet speech, proper nouns and code identifiers. Repeated passes measure timing variation, not new independent accuracy evidence. Do not fine-tune on the test set.

Explain: Why is changing a WAV header insufficient to resample? Why must a lazy generator be consumed before timing ends? Why does loading the model per sentence harm responsiveness? Why can WER exceed 100%? Which tests require actual devices? Why do Unicode marks matter in Hindi? The code and tests provide concrete examples for each answer.

## Desktop state, resource ownership and safety

Rust maintains loading → ready → recording → transcribing → ready, plus cancellation/error paths. React observes snapshots/events; it does not own microphone state. A session identifies each recording, while a worker generation identifies each Python lifetime. Checking both prevents valid but stale JSON from becoming a new transcript. The audio callback only downmixes into bounded storage; disk snapshots and inference must not hold its lock or the driver callback may stall.

NDJSON means one JSON object per line. `worker.py` flushes a response per command; diagnostics cannot enter stdout. Cancellation during native inference kills/reloads the worker because the current call is not cooperatively interruptible. That costs model startup but actually stops computation. Windows jobs terminate orphaned children; the named mutex prevents two instances racing on settings and capture cleanup.

Creating a WebView can run frontend code immediately. In `main.rs`, registering `Shared` before constructing either window makes the first `snapshot` command valid without a timing-based retry. Window lifetime also differs from application lifetime: closing the main window leaves the hidden indicator, so its close handler requests full application exit and the exit handler stops the worker. The installed-app checks exposed both boundaries even though unit tests passed.

A focused window and a specific field inside it are different identities. Window-only checks can type into the wrong browser field. `native.rs` compares opaque field runtime IDs and process metadata, rejects passwords and reads no field text. COM initialization belongs to the calling thread. SendInput success means input was queued, not proof that an editor stored it. The focus/input race explains opt-in insertion and the manual-copy path.

## Lessons from measured chunks

On the 11-second JFK clip, independent 4/8/12-second windows with beam 1 and both VAD settings produced zero normalized WER/CER. Four-second boundaries changed punctuation and increased total inference work. Punctuation-stripping metrics hide formatting differences. VAD needs silence/noise/quiet-speech examples to evaluate fairly; this speech-heavy clip cannot establish its quality. Rolling overlapping preview is a different experiment from independent chunks.

Windows peak working set covers the entire experiment process across configurations. It is not per-model memory and excludes Rust/WebView2. Repeating one clip estimates timing variation, not general English/Hindi/Hinglish accuracy. No fine-tuning or LLM claim is justified without licensed held-out examples and meaning-preservation checks.

Practice explaining stale-result safety, callback lock contention, COM/handle ownership, final latency with preview on/off, and installer verification on a machine without Python. Current code provides concrete examples.

## Input delivery is not text acceptance

The actual insertion fixtures exposed a boundary that a successful `SendInput` count cannot verify: the target app can receive input events and still produce different text. `native.rs` now uses synchronous character messages only for a verified Unicode RichEdit window and paced per-character input elsewhere. It checks field identity and cancellation between characters. The fixed-text helper verifies the result in its own generated documents; application code never reads target text. Already delivered characters cannot be undone safely by Cancel.

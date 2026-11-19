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

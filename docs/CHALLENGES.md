# Actual challenges and verification

## C1 — Device metadata changed between assumptions and installed API

**Component:** microphone enumeration.

**Expected behavior:** list input devices with their selectable indexes.

**Actual behavior:** the first real `devices` command returned an error.

**Evidence/error:** `dict() got multiple values for keyword argument 'index'` with sounddevice 0.5.6.

**Root-cause hypothesis:** device metadata already contains an index.

**Investigation:** inspected the returned metadata and changed construction to `dict(device, index=i)`.

**Verified cause:** the installed device dictionaries contain index, conflicting with the original explicit keyword plus unpacking.

**Alternatives considered:** remove the native key first; use the native index alone. Both introduce unnecessary dependence on a particular metadata version.

**Solution implemented:** copy metadata then explicitly set its enumerated index. Add a native-index regression test.

**Verification:** real `devices` command succeeded and listed 15 input entries, including the Realtek microphone array. The regression test passes. This does not verify physical capture.

**What I learned:** a mocked empty device list cannot catch changes in real device metadata; run read-only integration checks early.

## C2 — Missing tokenizer could bypass intended offline loading

**Component:** model initialization.

**Expected behavior:** offline commands must not fetch missing model components.

**Actual behavior:** upstream constructor inspection showed a fallback to `Tokenizer.from_pretrained` when a local model directory has no tokenizer.json, even with local_files_only passed to model resolution.

**Evidence/error:** the fallback is visible in faster-whisper's constructor; no unauthorized download was observed during our checks.

**Root-cause hypothesis:** the offline model-resolution option does not cover the separate tokenizer fallback.

**Investigation:** checked the upstream implementation and added an incomplete-local-directory test.

**Verified cause:** the fallback branch lacks the local-only argument. This is a code-inspection finding, not a reproduced network incident.

**Alternatives considered:** mutate global offline environment variables; rely on the upstream option. Global state complicates commands that explicitly permit downloading, and the latter leaves the fallback open.

**Solution implemented:** resolve the model with the existing downloader, require a local tokenizer.json, then initialize from the complete directory.

**Verification:** the incomplete-model test fails safely before initialization; cached real-model inference was rerun separately.

**What I learned:** an offline flag must cover every component loader, not just weights.

## Verification scope

Host: Windows build 26200, Python 3.10.11, Intel Core i5-12450HX, 12 logical CPUs, approximately 11.7 GiB RAM. Installed faster-whisper 1.2.1, CTranslate2 4.9.0, sounddevice 0.5.6, jiwer 4.0.0 and NumPy 2.2.6. Development date: 2026-11-19.

Hardware-independent suite: 30 tests passed. This includes real PyAV decoding/resampling at 16/44.1/48 kHz, stereo downmix, invalid/empty audio, exact silence, model reuse, metrics, Unicode, malformed manifests, recording cleanup and CLI error/cancellation paths. PortAudio capture is mocked in these tests. `pip check` reported no broken requirements.

Real integration: downloaded the [Whisper JFK FLAC test clip](https://github.com/openai/whisper/blob/main/tests/jfk.flac), 11 seconds, and ran multilingual tiny and base on CPU int8, beam 5, language en, three passes each. The reference uses the known words of the recorded speech, with shared normalization. Both models scored WER 0 and CER 0 on this single clip. This widely used test clip is not an independent held-out benchmark; results establish pipeline operation only, not general speech quality or superiority of one model. Initial downloaded runs are saved locally in ignored reports/jfk-comparison.json. The base initialization in that run includes a download (43.35 seconds), so it must not be presented as cached startup time. The first standalone tiny run took 1.66 seconds of inference, with download-inclusive initialization of 27.42 seconds.

After the tokenizer safeguard change, the same comparison succeeded without `--download`. Raw local results are in ignored reports/jfk-offline-comparison.json. Cached results (three passes each, no hidden warmup):

| Model | Load seconds | Median inference seconds | Aggregate RTF | WER / CER |
| --- | ---: | ---: | ---: | ---: |
| tiny | 0.428 | 0.361 | 0.033 | 0 / 0 |
| base | 0.370 | 3.139 | 0.287 | 0 / 0 |

These are observed runs under uncontrolled system load, with one short clip. Model load includes resolution of cached weights. No p95 is reported for three observations. No conclusion about general multilingual accuracy or memory efficiency follows from these numbers.

Physical microphone recording, driver disconnection, GPU, Hindi/Hinglish quality, peak memory, CPU utilization and interactive capture latency remain unverified. No microphone recording was activated during development. The next acceptance check is an explicit interactive recording and transcription using the README commands. Do not begin the desktop milestone before that check succeeds.

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

At the Milestone 1 handoff, physical microphone recording, driver disconnection, GPU, Hindi/Hinglish quality, peak memory, CPU utilization and interactive capture latency were unverified. No microphone recording had been activated. The owner subsequently expanded scope to the remaining milestones; current verification is recorded below without treating that expansion as hardware proof.

## C3 — Cargo registry freshness requests stalled

**Component:** desktop dependency resolution.

**Expected behavior:** Cargo resolves dependencies and begins compiling.

**Actual behavior:** repeated registry updates stalled while direct HTTPS downloads could work. Offline resolution reported missing package metadata.

**Evidence/error:** Cargo reported `transfer too slow ... 0 bytes`; a direct metadata request also produced `[Errno 11001] getaddrinfo failed`. DNS resolution later succeeded. These observations do not prove a single underlying network cause.

**Root-cause hypothesis:** intermittent network/DNS problems affected registry freshness requests.

**Investigation:** checked connectivity, retried with bounded network timeouts, and compared offline resolution. A temporary, ignored recovery script fetched missing sparse-index metadata from the official index.crates.io endpoint using normal TLS verification.

**Verified cause:** metadata was absent from the local cache and normal registry updates stalled. The underlying network failure is not fully diagnosed.

**Alternatives considered:** wait indefinitely, use an unofficial mirror, or abandon native verification. None provides useful local evidence.

**Solution implemented:** populate missing official metadata, select the current stable Tauri v2 family, generate Cargo.lock with Cargo, then use normal locked downloads/compilation. No TLS checks or crate checksum verification were disabled. The recovery helper is not part of the application or required on a healthy connection.

**Verification:** Cargo generated the lockfile, downloaded the official crates, compiled the native test executable and passed all four Rust tests. Strict Clippy also passed before the subsequent startup-conflict handling review.

**What I learned:** distinguish dependency acquisition failures from source/compiler failures; preserve a lockfile and report network hypotheses honestly.

## C4 — Device-name comparison required error-aware matching

**Component:** CPAL microphone selection.

**Expected behavior:** choose the configured input by its name.

**Actual behavior:** native compilation rejected comparing `Result<&str, &DeviceNameError>` with `Ok(name)`.

**Evidence/error:** Rust E0369: DeviceNameError does not implement PartialEq.

**Root-cause hypothesis:** equality on Result also compares the error variant.

**Investigation:** checked CPAL's return type and matched only successful names.

**Verified cause:** Result equality requires both success and error types to support PartialEq.

**Alternatives considered:** manually match each Result; invent equality for a foreign error type. The latter is inappropriate.

**Solution implemented:** compare `d.name().ok().as_deref()` with `Some(name)`, skipping devices whose names cannot be read.

**Verification:** native compilation proceeded and four Rust tests passed.

**What I learned:** compare the meaningful successful value, not an error-bearing transport type.

## Expanded implementation verification

- Python: **47 passed**; dependency consistency check passed.
- Rust: **4 passed** (valid WAV, atomic settings replacement, corrupt settings and stale-target refusal). These do not establish a physical microphone or target editor works.
- Frontend: TypeScript/production build passed. Runtime frontend dependencies reported zero known vulnerabilities in npm audit; this is not a complete security audit.
- Source NDJSON worker: real model load/transcribe/shutdown succeeded with VAD.
- Frozen worker: real offline tiny-model transcription succeeded with VAD and terminology hints; exit 0, ready/transcript events, measured inference 0.481 seconds. Packaged runtime folder measured 254,995,766 bytes before desktop bundling.
- Screenshot-only Edge rendering inspected. A remote-debugging browser launch was rejected by automatic approval review as “blocked by policy”; no more specific reason was returned. That action was not bypassed. Screenshot rendering does not exercise the Tauri bridge.

Twenty cached tiny-model passes of the same JFK clip, CPU int8, beam 5, language en: median inference **0.399 s**, p95 **0.443 s**, aggregate RTF **0.0369**, normalized WER/CER **0/0**. This is repeated timing evidence on one familiar clip, not broad accuracy evidence. Raw local results: ignored reports/tiny-20passes.json.

Independent chunk experiments (tiny, int8, beam 1, same clip):

| VAD | Chunk seconds | Aggregate RTF | Median chunk inference seconds |
| --- | ---: | ---: | ---: |
| off | 4 | 0.0946 | 0.329 |
| off | 8 | 0.0614 | 0.337 |
| off | 12 | 0.0332 | 0.365 |
| on | 4 | 0.1216 | 0.332 |
| on | 8 | 0.0624 | 0.343 |
| on | 12 | 0.0344 | 0.379 |

All normalized WER/CER values were zero, but punctuation changed at shorter boundaries. Peak process working set was 243,183,616 bytes without VAD and 266,878,976 bytes with VAD in separate experiment processes. These peaks include the complete experiment process, not the desktop, and do not isolate per-configuration memory. Source reports are ignored reports/chunk-baseline.json and reports/chunk-vad.json.

## C5 — Installer bundling encountered a live-file lock (unresolved at pause)

**Component:** Windows NSIS bundling.

**Expected behavior:** produce a successful per-user installer after compiling the executable.

**Actual behavior:** release compilation succeeded and makensis produced a file, but Tauri's bundle command returned Windows error 32.

**Evidence/error:** `The process cannot access the file because it is being used by another process. (os error 32)`. The release app had been launched for startup inspection while bundling was still running.

**Root-cause hypothesis:** launching the executable during bundling held a file that the bundler needed. This was an avoidable sequencing mistake.

**Investigation:** confirmed the live app and installer artifact existed; stopped the app when the owner requested a pause.

**Verified cause:** a file-in-use error is confirmed. The exact locked file and successful mitigation have not yet been verified.

**Alternatives considered:** do not terminate unrelated processes or disable security software. Retry with this app closed first.

**Solution implemented:** no verified fix yet; the app was closed and the retry is the first resume task.

**Verification:** pending. Do not report the existing installer artifact as a successful bundle.

**What I learned:** finish bundling before launching the executable under inspection. Compilation success, artifact existence and successful installer production are different checks.

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

### C5 resume result (2026-11-20)

The closed-app retry completed successfully (exit 0), producing a 70.29 MiB NSIS installer. It also included the application license and third-party notices through the release resource map. Silent per-user installation into the ignored `artifacts/install-smoke` directory returned 0; the desktop executable, frozen worker and uninstaller existed, and installed license/notice hashes matched the source files. Silent uninstall returned 0 and removed the install directory and uninstall registry entry. These are checks on the development machine, not a clean Windows machine. The earlier exact locked file remains unidentified; a successful closed-app retry supports the sequencing mitigation without proving which process held that file.

## C6 - Initial native snapshot arrived before runtime registration

**Component:** desktop startup and Rust IPC.

**Expected behavior:** the initial snapshot supplies saved preferences and current engine state.

**Actual behavior:** the installed UI showed ``state not managed for field `state` on command `snapshot` ``. The real worker subsequently loaded and the UI received the ready event, but preferences stayed unloaded and the connection error remained visible.

**Verified cause:** configured WebViews can execute their frontend before Tauri's setup callback registers `Shared`. The initial snapshot failed before `app.manage(shared)`; later worker readiness did not retry that snapshot. UI Automation inspected the actual installed app with renderer accessibility enabled, without a remote debugging port.

**Solution implemented:** configure both windows with `create: false`, then construct them from their existing configuration after registering the complete runtime in setup. This preserves the current initial snapshot protocol without retry timing or another dependency.

**Verification:** Rust tests and strict Clippy passed after the change. A native debug build with embedded production frontend assets showed model `base`, an enabled record button and `Ready. Your microphone is off.`, with preferences loaded and no connection error banner. The first direct debug/development-server launches showed a blank WebView despite the dev server returning HTTP 200; their navigation failure remains undiagnosed and is separate from the passing embedded-assets check. Final rebuilt installer recheck remains pending.

## C7 - Hidden indicator kept the application alive after main-window close

**Component:** desktop shutdown and worker lifetime.

**Expected behavior:** closing the main window exits the app and its worker.

**Actual behavior:** a normal `CloseMainWindow()` request succeeded, but the desktop and frozen worker remained alive after five seconds.

**Verified cause:** Tauri only exits automatically after the last window closes; the hidden indicator still existed. The application's worker cleanup ran only on process exit. Force-terminating this test desktop did terminate its supervised frozen worker; that separate crash-path observation did not establish normal-close behavior.

**Solution implemented:** the main window's close-request event explicitly requests application exit, allowing the existing exit handler to terminate the worker. No background-only mode is introduced.

**Verification:** Rust tests and strict Clippy passed after the change. A normal close request on the updated native debug app returned true, the app exited within the five-second observation window, and its source worker was no longer running. Final rebuilt installer recheck remains pending.

## C8 - First owner-guided microphone transcript did not match speech

**Component:** interactive native microphone-to-ASR flow.

**Observed result (2026-11-20):** the owner reported that recording stopped and a transcript appeared, but the transcript did not match what they said. This establishes one user-reported physical start/stop/result flow, not acceptable recognition quality or complete microphone acceptance.

**Local inspection:** the native UI was ready again with Copy enabled and the transcript-ready status; no app-owned capture session directories remained. Active preferences showed base, INT8, automatic language detection, Windows default input and verbatim output. Preferences were inspected without saving changes, and the Dictation page was restored. No audio or transcript was copied into a report or artifact.

**Cause:** not established. The original capture was removed by normal completion, so it cannot be replayed for a same-clip comparison. The owner has been asked for the non-private spoken/displayed comparison and confirmation of indicator visibility. Do not attribute the mismatch to language detection, microphone quality, VAD or model capacity without evidence.

**Next:** use the reported mismatch to choose a controlled follow-up. Record cancellation and shortcut/target-app checks separately. No fine-tuning, LLM rewriting or model download has been justified by this single result.

**Follow-up:** the owner supplied a spoken/displayed comparison showing errors across the sentence, beyond proper-name spelling. The text is not persisted here as transcript history. Read-only device enumeration reported a default Realtek microphone array and a connected Nirvana headset; this alone does not establish which physical source produced the failed capture or why recognition failed. The owner had closed the app. Only the saved language preference was changed from automatic to English using an atomic replacement, and the same embedded-assets native debug app was reopened. UI inspection confirmed English, base, INT8, default input and verbatim, a ready/off microphone and zero capture directories. The owner has been asked to repeat the same non-private sentence from the same speaking position. No new recording was started by the agent, and no recognition improvement is claimed before the result arrives.

**English-only result:** the owner again reported substantial sentence-level errors, and confirmed the recording indicator appeared during capture and disappeared afterward. Explicit English did not resolve the reported mismatch. Native status returned ready/transcript-ready with Copy enabled and zero capture directories after completion.

**Confirmed input configuration mismatch:** Windows WASAPI default-input enumeration reported the Realtek microphone array; CPAL's Windows default-input implementation uses the default capture endpoint. The owner confirmed they intended to use the Nirvana headset. SONIQ's saved input was still Windows default. The test app was closed normally; its frozen worker was no longer present afterward. Only the microphone preference was changed to `Headset (Nirvana Ion ANC)`, preserving English/base/INT8/VAD/verbatim, and the app was reopened. The native Preferences dropdown confirmed the headset selection, and recording remained off. A headset repeat test is pending. This verifies a configuration mismatch, not yet that correcting it fixes recognition. The proposed additional in-memory audio-level/VAD diagnostic was deferred before implementation or activation.

**Headset result:** the owner reported another inaccurate transcript after explicitly selecting the headset. Selecting the intended input has therefore not established a quality fix. No quality metrics are assigned to separate spoken repetitions.

**Next diagnostic prepared:** ignored `artifacts/microphone-diagnostic.py` uses existing sounddevice/NumPy/faster-whisper dependencies. It checks the named headset's WASAPI format and loads cached base/INT8 offline, then requires the owner to type `RECORD` in a visible console. Only afterward does it open a bounded ten-second stream, display `SPEAK NOW`, close the microphone, measure peak/RMS/near-full-scale samples, offer explicit local playback, and transcribe the same in-memory samples in English with VAD on/off. It writes no audio or report files and does not start another recording automatically. Ctrl+C during capture closes the context-managed stream and discards partial blocks. This uses PortAudio rather than the desktop CPAL capture path, so its results must be distinguished from native desktop evidence.

**Preflight:** `--check` passed for `Headset (Nirvana Ion ANC)`, Windows WASAPI, native 16 kHz mono; cached base initialized without a microphone stream. Python suite again passed 47 tests and venv dependency check passed. The visible diagnostic process was launched awaiting explicit owner input; playback, actual diagnostic capture and same-clip VAD results remain pending. No extra dependency or model download was added.

**Owner diagnostic follow-up:** the owner reported completing the diagnostic but said it was still not fully accurate. The diagnostic process was no longer running when checked afterward. No recording or report was retained, so the agent cannot recover its console results or replay that capture. The owner has been asked for the actual diagnostic text and whether local playback was clear; if the text is unavailable, whether errors affected only names or also ordinary words. Signal measurements, VAD comparison, playback clarity and the exact remaining error pattern are not yet established. No further setting/model change is justified by the percentage description alone.

**Direct-result diagnostic:** after the owner said they had forgotten the output, a standard-library Tk window was added to the ignored diagnostic script. It opens capture only on an explicit Record button, displays SPEAK NOW after the stream opens, supports cancellation/close, and returns results through its parent process stdout instead of saving files. The first ten-second ordinary-sentence capture returned text matching the displayed sentence with VAD on and off. Peak was 0.54395, RMS -24.04 dBFS and near-full-scale fraction 0%. These are observed scalar measurements on one clip, not a broad accuracy or hardware certification. Playback clarity was not reported. This first diagnostic imported an older installed source copy, identified in C9; the unhinted recognition settings were base/INT8/English/beam 5.

**Current-source name comparison:** after correcting the diagnostic import, another explicit ten-second headset capture compared VAD on/off without hints and VAD on with hints for the personal name and then-current product name, using one reused base/INT8 model and identical samples. Both unhinted passes misrecognized the personal name. The hinted pass recognized the intended personal and product names, while inflection/article differences remained. Peak was 0.60550, RMS -21.04 dBFS and near-full-scale fraction 0%. Disabling VAD did not correct the name. These observations support trying the two vocabulary terms for this owner; they do not establish a general correction benefit. Only those local terminology preferences were added, preserving English and the explicit headset. No LLM, new model, download or transcript history was added. Both diagnostic windows were closed and their in-memory capture discarded.

**Native release follow-up:** the final rebuilt release app is open with headset/English/base/INT8/verbatim and saved terminology. The owner is asked to record an ordinary sentence plus the name-containing sentence after the timer starts counting, stop and leave the current result visible for transient UI inspection. That native result and any capture-start timing diagnosis remain pending.

**Native release result:** after the owner reported completion, transient UI inspection showed both intended names and the remainder of the requested sentence recognized correctly, with the leading word absent. There is no retained audio or independently checked reference to establish why that word was absent. The app returned to ready/transcript-ready, Copy was enabled and capture session count was zero. Do not infer broad quality or a capture-start root cause from this single result. A separate owner-guided cancellation check has been requested.

## C9 - Diagnostic imported an older installed project copy

**Observed failure:** the first name-hint diagnostic failed with `Transcriber.transcribe_samples() got an unexpected keyword argument 'hotwords'` before returning its ASR comparison.

**Verified cause:** executing the script from `artifacts/` put that directory first on Python's search path. The venv contained a non-editable older project package without the hotwords parameter, while imports from the repository root resolved current source. Inspection confirmed the differing signatures. The frozen worker's existing PyInstaller analysis entries pointed to the current checkout for worker/asr/audio/text, so this evidence does not show that the desktop worker used the older installed copy.

**Resolution:** the ignored diagnostic explicitly puts its own checkout root first on the module search path and reports its ASR source before enabling recording. Its preflight and successful name-hint run resolved the checkout's ASR module. An attempted offline, no-dependency editable relink failed during metadata preparation with `invalid command 'bdist_wheel'`; no package uninstall/install occurred. No dependency setup was repeated to address this diagnostic-only import issue.

## Final local release smoke checks (2026-11-20)

The rebuilt installer containing the startup/shutdown fixes completed bundling with exit 0 (70.29 MiB). SHA-256: `7D96533F3C569269E3679070F315FED017F6C9146757AA105A15DAB361853933`.

- Silent per-user installation into ignored `artifacts/install-smoke` passed. Executable, bundled frozen worker and uninstaller existed; installed license/notice hashes matched the source.
- Native UI inspection of that installed app observed ready/microphone-off state, enabled recording and no initial connection error.
- A second instance exited successfully, leaving one desktop process.
- Closing the main window exited the app and its frozen worker within the five-second check window.
- Force-terminating a separate idle installed desktop also stopped its frozen worker within the five-second check window. No microphone was activated in these automated lifecycle checks.
- Silent uninstall returned 0 and removed the test install directory and uninstall registry entry. Owner preferences retained the same hash.
- Final checks: 47 Python tests, venv `pip check`, four Rust tests, strict Clippy, Rust formatting and TypeScript/Vite production build passed.

The local scripts are ignored under artifacts. These results verify this development-machine installation and idle lifecycle, not clean-machine installation, capture-in-progress shutdown, signing, complete redistribution compliance, remote CI, application-field insertion or public release. The release executable was reopened only after bundling and uninstall checks finished, for the explicitly guided native speech test.

## Interactive cancellation and insertion acceptance (2026-11-20)

The owner performed the requested native Record/speak/Cancel check. Transient inspection confirmed the cancelled status, microphone off, enabled Start, disabled Copy, empty transcript preview and zero capture session directories. This verifies this observed cancellation path; cancellation during active model inference and recording-in-progress application shutdown remain separate checks.

For target-app checks, automatic insertion was temporarily enabled through the actual Preferences checkbox and Save command, preserving the owner's other preferences. A dedicated, initially empty the dedicated ignored insertion-test document was opened in Notepad; no existing user document was overwritten. The owner has been asked to use hold-to-talk in that document and keep it focused through transcription. Result pending. Restore automatic insertion to off after the interactive checks, using the actual Preferences flow. No microphone was started by the agent.

## C10 - Successful input delivery corrupted actual target text (2026-11-25)

The automated fixture called the production `native.rs` field/insertion functions, without ASR or a microphone. Browser short-text insertion, focus-change, password and read-only checks initially passed. Modern Notepad reported successful input delivery but its saved fixture contained the then-current product name followed by repeated periods instead of the fixed 25-character test string. Pacing generic keyboard input alone did not establish a fix for longer Notepad text. A 780-character browser fixture also differed from the requested text after a whole-string batch.

The confirmed boundary is queued input delivery versus target acceptance; the underlying Windows/provider behavior is not fully diagnosed. Modern Notepad now uses synchronous `WM_CHAR` delivery only after verifying its Unicode RichEdit native window and process. Other fields use one Unicode scalar per input batch with 20 ms pacing. Both paths revalidate the focused field, modifiers and session between characters, and report possible partial insertion on interruption. Cancellation cannot undo text already delivered.

The final no-microphone fixture run passed short ASCII, Hindi/accent/emoji and 780-character text in Edge and modern Notepad, plus 133-character browser text, same-window focus change, password/read-only refusal, native insertion cancellation after its first character and a focus change during native insertion. Two additional Rust tests verify control-character/oversized-text refusal and cancelled-session refusal. All six Rust tests, strict all-target Clippy and formatting passed; 47 Python tests, venv pip check and the frontend build passed. These are native text-delivery checks, not a completed microphone/shortcut-to-ASR insertion check.

Test-harness fixes were also required: select actual editable controls instead of matching label text, allow asynchronous accessibility updates before comparing fixture content, and compare the foreground window's root ancestor when an editor child is foreground. Some runs correctly stopped for missing foreground focus. A simultaneously running development watcher also reloaded the app when fixtures/source changed; final acceptance was run with that watcher stopped. No broad root cause is assigned to all transient focus refusals.

## C11 - Fresh Windows runner installer smoke check (2026-11-25)

GitHub Windows checks passed for the initial public source and the installer-workflow commit. A separate fresh hosted Windows runner successfully packaged the worker and NSIS installer, but its first smoke check failed at uninstall cleanup. The check waited for directory removal, then immediately required registry removal. Its final error did not distinguish the two remnants and could hide an earlier failure. The check now waits for both directory and registry removal and emits separate resource and offline-worker evidence. A successful rerun is still required; do not infer uninstall correctness from the script change.

This runner has preinstalled development/runtime components, so even a successful run is not clean consumer-Windows or hardware acceptance. No microphone is used and no model download is permitted by this smoke check.

## Development WebView follow-up (2026-11-25)

A real `npm run desktop` launch was inspected through a temporary loopback-only WebView debugging endpoint. The first inspection found an interactive document with an empty React root. After reload, both windows reached complete documents with populated React roots; the main window showed an enabled Start recording button and disabled Copy, demonstrating a working native state flow. The captured console showed only a missing favicon, not an established cause of the earlier blank startup. No permanent navigation fix or cold-start guarantee is claimed. The app/dev watcher were closed afterward; the debugging endpoint was temporary. No microphone was activated.

### C11 follow-up results

The updated production-source commit `8fc5e10` passed Windows tests/build/strict Clippy at https://github.com/MuditTandon20/SONIQ/actions/runs/45583427752 and the fresh hosted Windows installer workflow at https://github.com/MuditTandon20/SONIQ/actions/runs/45583459268. The latter packaged, installed, verified notices, exercised the installed frozen worker with external Python removed from PATH and an empty offline model cache, and uninstalled successfully. It did not use a microphone or download weights.

The rebuilt local installer also passed those isolated offline checks. Installed native readiness, single-instance protection, normal close/frozen-worker exit and forced idle close/frozen-worker exit passed. A subsequent uninstall from the OneDrive workspace smoke location left the installation/registration intact on its first attempt; another silent invocation exited 0 and removed both. The same native lifecycle and uninstall sequence then passed from a newly created TEMP installation outside OneDrive. The exact cause of the intermittent workspace-path uninstall remains undiagnosed; do not claim that OneDrive was proven responsible or that every uninstall path is certified.

The inventory matcher initially missed tqdm's existing `licenses/LICENCE` because it checked only the American spelling. It now checks both spellings and identifies only CTranslate2 as lacking a license file within its checked package/metadata directories. Supplementary versioned texts remain installed alongside the worker. The report still does not establish a complete license audit.

## Licensed Hindi pilot and real frozen-worker integration (2026-11-25)

Twenty clips (234.66 seconds; 527 normalized reference words) were taken in deterministic first-WAV-member order from the Hindi test archive of [Google FLEURS](https://huggingface.co/datasets/google/fleurs), revision `70bb2e84b976b7e960aa89f1c648e09c59f894dd`, licensed CC-BY-4.0. The publisher's independently supplied raw transcriptions were used as gold references; this continuation did not manually listen-audit them. This is a small read-speech subset, not a representative dictation or Hinglish acceptance set. The rows API returned HTTP 500 and the first-rows API returned 501; the pinned original TSV and streaming TAR archive were accessible and required only standard-library tools. Only the first 20 matched WAV members were downloaded, without selection by recognition quality.

| Model / pipeline | VAD | Corpus WER | Corpus CER |
| --- | --- | ---: | ---: |
| tiny / current source | off | 107.97% | 95.73% |
| base / current source | off | 106.64% | 91.00% |
| small / actual CLI | off | 62.43% | 29.05% |
| small / frozen production worker | on | 57.50% | 25.30% |

All used CPU, INT8, explicit Hindi and beam 5, with verbatim output and no vocabulary hints. WER can exceed 100% because insertions count as errors. A scalar-only diagnostic on one base-model clip confirmed a multilingual model, matching decoded/publisher audio duration and nonzero audio levels, but its returned letters were largely Arabic script rather than the gold Devanagari script. No general root cause or automatic correction is established.

Tiny/base used cached models. The small comparison invoked the actual CLI with explicit `--download`, fetching the existing supported Systran small model; it did not add an application dependency or change owner preferences. The frozen worker then loaded that cached small model offline once and completed twenty real NDJSON transcription commands with VAD enabled, preserving verbatim text. It shut down afterward. The source and frozen runs differ in both pipeline and VAD setting, so this is not an isolated VAD-benefit experiment. Some runs overlapped packaging CPU load; do not use their timings as an isolated performance comparison.

Audio, public gold-reference manifest, clip hashes/provenance and aggregate reports remain ignored under recordings/, artifacts/ and reports/. Recognition hypotheses were discarded from persisted reports; no owner recording, microphone activation, audio upload or transcript-history feature was involved. The reported improvement does not establish acceptable Hindi quality: errors remain high even in the small/VAD packaged-worker run. Hinglish, broader speakers/devices and owner-guided microphone quality remain open. No LLM processing or fine-tuning is justified by these results alone.

## Final automated checkpoint (2026-11-25)

- Latest code checks passed remotely at `61ce119`: https://github.com/MuditTandon20/SONIQ/actions/runs/45586073415. Production code and installer smoke evidence remain linked above.
- After correcting the inventory wording, a final unsigned installer rebuilt successfully: 73,712,022 bytes; SHA-256 `2DA24C9355548BF8F5C99313B223A28DBBE9AE09D287E16C3AD86C30181AA5B3`. Its install, all notice hashes, installed frozen-worker offline missing-model behavior without external Python, and uninstall passed locally. No microphone or model download was used in this installer check.
- Python checks again passed: 47 tests and venv pip check. Generated recordings, manifests, reports and diagnostic scripts were explicitly confirmed ignored by Git.
- A native development launch with `npm run dev -- --force` as Tauri's before-dev command reached the ready/off state without requesting a reload, 24.18 seconds after the desktop process started. The app closed normally with exit 0 and no surviving app/worker. A WebView shutdown class-unregistration diagnostic (1412) appeared on stderr; no cause is established for that message or the older blank launch. This verifies the observed forced-reoptimization launch, not every cold-machine startup.
- Trusted signing, an actual clean consumer Windows machine, full exact-native-build redistribution evidence, broader language quality and the remaining owner-guided capture checks remain distinct gates. The public GitHub repository contains source, not a verified public installer release.

## Guided transcription cancellation checkpoint (2026-11-25)

The rebuilt release reached ready/off with enabled Start, disabled Copy, empty preview and zero capture directories. A no-recording helper waited for the owner to click Record and Stop, intending to invoke Cancel only after observing the native transcription phase and verify old-worker exit, replacement-worker readiness and discarded preview/capture state. Its 180-second observation window expired without a completed guided check. It did not start a microphone and is no longer armed. No transcription-cancellation, physical shortcut-to-target or recording-in-progress shutdown pass is claimed. Rearm explicitly before a subsequent owner-guided attempt.

## C13 - A saved disconnected headset blocked recording (2026-11-25)

The owner requested automatic audio-device handling and a shortcut with at most two keys. Inspection confirmed that a named input used only an exact current-device match; an absent saved headset returned an error instead of consulting the available Windows default. The empty/default setting already queried Windows dynamically, but did not help this owner's persisted named-headset preference.

The source now rechecks devices for every explicit recording and falls back to the current Windows input when the preferred device is absent or cannot be enumerated. It keeps the saved preference so a later reconnect is selected again. It reports the actually opened input in the recording status, refreshes Preferences metadata on app focus/page entry and preserves an unlisted preferred option in the UI. It does not open a microphone for device discovery, retry a different device in the middle of capture, or silently start a new recording after disconnection.

Ctrl+Space is the new two-key default, and the owner's saved shortcut was atomically changed to that value with the app closed. All other saved preferences were compared and preserved. Existing custom shortcuts remain readable without migration. Nine Rust tests, strict all-target Clippy, 47 Python tests, venv pip check and TypeScript/Vite build passed. The new device-policy tests use value-only device stand-ins; physical unplug/replug capture and two-key hold-to-talk still need owner-guided evidence. Updated packaging/native startup results are recorded next when observed.

### C13 packaged/runtime results

Release bundling returned 0 and produced an unsigned installer with SHA-256 `42BBDC36B8873664B9BA99D1A29F936281AE82CC0394D3FB10BAC3AE7AA0C18F`. Isolated install, notice hashes, installed frozen-worker operation without external Python and with an empty offline model cache, and uninstall passed. Windows CI passed for source commit `1cb4acb`: https://github.com/MuditTandon20/SONIQ/actions/runs/45637763642.

The rebuilt native app reached ready/off. Ctrl+Space registered during startup, appeared in its live shortcut caption and loaded as the saved Preferences value. The saved headset preference remained selected; it was currently absent from the enumerated device metadata and was displayed with automatic-fallback text. The new automatic-input help was visible. The Dictation page was restored without starting capture. Physical fallback audio, reconnect capture and two-key hold/release remain owner-guided checks, not inferred passes from UI metadata or selection-policy unit tests.

Inspection itself required targeting the actual Tauri main window by its title and class: `.MainWindowHandle` sometimes chose the framework's visible `Tao Thread Event Target` instead and gave false empty-control snapshots. The native microphone dropdown exposed ValuePattern, not SelectionPattern. The ignored diagnostic helpers were corrected accordingly; these were observer issues, not demonstrated app startup failures. The app remained ready with disabled Copy, an empty preview and zero capture directories.

## C14 - Complete SONIQ rebrand (2026-11-26)

The GitHub repository was renamed in place to https://github.com/MuditTandon20/SONIQ and the local Git origin was updated. Current project files were scanned for obsolete branding after updating app/UI titles, namespace/package metadata, worker/executable names, installer workflow, scripts, test imports and documentation. The Python package directory was renamed to `soniq/`. Historical product-name hint and corrupted-fixture descriptions were generalized rather than claiming that the old recordings recognized the newly chosen SONIQ name. Historical metrics, timestamps and hashes were preserved.

The existing venv's old project distribution was replaced with the SONIQ wheel without reinstalling application dependencies. Forty-seven Python tests, venv pip check, nine Rust tests, strict all-target Clippy and TypeScript/Vite build passed. The installed `soniq` entry point and Python import worked from TEMP outside the checkout. Full packaging rebuilt `soniq-worker.exe`, `soniq.exe` and the SONIQ NSIS installer successfully. PyInstaller warned that optional ONNX quantization tooling was absent; the application uses inference/VAD rather than that tooling, and the frozen-worker runtime still requires its own explicit smoke check.

The owner explicitly authorized discarding the current preview and restarting. The previous app closed normally, and saved settings were atomically copied to the new identifier's settings directory with all non-vocabulary preferences preserved. The vocabulary product name was updated to SONIQ; no recognition benefit for that new hint is claimed. No audio, transcript or WebView data was migrated and no microphone was activated. Final package and native checks are recorded below when observed.

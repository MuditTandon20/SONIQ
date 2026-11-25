# Roadmap

## Current continuation status - 2026-11-25

- The owner authorized the remaining verification work, then requested automated checks before further guided microphone tests. No microphone was activated in this continuation.
- Public-source Windows CI passed. A new manually dispatched installer workflow built on a fresh hosted Windows runner; its first cleanup check failed. The check now waits for both directory and registry removal. Rerun and local updated-installer evidence are pending.
- Native insertion fixtures exposed real Notepad/browser text corruption despite successful input-delivery counts. The source now uses verified synchronous Unicode RichEdit messages for modern Notepad and paced per-character Unicode input elsewhere, with field/session revalidation. The final fixed-text checks passed in Edge and Notepad, including 25-, 133- and 780-character cases, Hindi/accent/emoji text, password/read-only refusal, focus-change refusal and interruption of native insertion. These do not replace a real shortcut/microphone/ASR-to-target acceptance check.
- Six Rust tests, strict all-target Clippy and formatting passed; 47 Python tests, venv `pip check`, frontend build and npm runtime audit passed. No application dependency was added.
- Automatic insertion was restored OFF through the real Preferences flow, preserving other preferences. The test app and dev watcher were closed. Do not assume historical PIDs or open test windows are current.
- Development WebViews rendered and the native engine reached ready after a reload during inspection. The earlier blank initial navigation still has no confirmed root cause; cold-start reliability is not closed.
- An exact-file worker inventory found missing CTranslate2/tqdm license texts; pinned upstream copies and MPL text are now mapped into installer notices. Full FFmpeg/native corresponding-source and redistribution review remains open. No signing certificate was found in current-user or local-machine stores. No public binary release is justified yet.

Resume pending automated installer/benchmark work from the newest evidence below. Remaining owner-guided gates are inference-time cancellation, recording-in-progress shutdown, physical shortcut-to-target insertion and broader microphone/Hinglish quality. A hosted CI runner does not establish clean consumer-machine or physical hardware acceptance.

## Repository publication - 2026-11-25

The owner authorized a public source repository at https://github.com/MuditTandon20/FlowForge-AI. Existing local history and the saved desktop lifecycle fixes are being published; generated installers, recordings, reports and model weights remain excluded. Pre-publication checks passed: 47 Python tests, venv `pip check`, four Rust tests and Rust formatting. Remote CI and a packaged public release still require separate evidence. The implementation continuation and remaining interactive checks below remain the resume point.

1. **Speech foundation:** CLI, WAV, offline ASR, reference evaluation and model reuse are implemented and tested. Physical microphone verification remains separate.
2. **Desktop:** Tauri/React, CPAL capture, shortcuts, worker supervision, preview, cancellation, atomic preferences and guarded insertion/manual copy are implemented. Local build/installation, native readiness, idle lifecycle and owner-guided speech start/stop have observed evidence. Remaining interactive and target-app checks are separate gates.
3. **Correction:** original recovery, optional whitespace/filler/paragraph cleanup and terminology hints are implemented. No LLM without measured improvement.
4. **Streaming investigation:** optional bounded rolling preview and Silero VAD are implemented; real chunk comparisons ran. True incremental decoding and stable cumulative partials are not claimed.
5. **Research:** same-clip model/chunk/VAD experiments, WER/CER, latency and process peak memory measurements ran. Broader licensed multilingual data and justified fine-tuning remain future work.
6. **Release:** packaging scripts, supervision, single-instance protection, cleanup and Windows CI are implemented. Development-machine installer/lifecycle smoke checks passed. Clean-machine testing, signing and publication need separate evidence.

The owner subsequently authorized the remaining milestones together. Desktop, cleanup, bounded preview, experiments and packaging are now being delivered. Code availability is separate from acceptance evidence; see CHALLENGES for actual results.

Remaining gates: native capture cancellation and recording-in-progress shutdown; shortcut and Notepad/browser insertion with focus/password refusal; broader consented Hindi/Hinglish evaluation; actual clean-machine installer, signing and public release checks. Recognition evidence is limited to the observed short clips and still requires review. No LLM or fine-tuning is justified without measured benefit and suitable data. Rolling preview does not claim incremental decoding, stabilized cumulative text or automatic endpointing.

## Resume checkpoint — 2026-11-20

The owner asked to save everything and pause until they say “continue”. Continue from here; do not rebuild the project from scratch.

Completed and verified:

- Python suite: 47 passed. `pip check` passed.
- Rust suite: 4 passed after the latest startup/shortcut conflict changes. Strict `cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings` passed.
- TypeScript and Vite production build passed; npm runtime audit reported zero known vulnerabilities.
- Real source and PyInstaller-frozen workers performed offline tiny-model transcription with VAD; terminology hints were also checked in the frozen worker.
- Actual CPU chunk/VAD and twenty-pass latency experiments are recorded in CHALLENGES. Raw data is in ignored reports/.
- Production Rust executable built successfully: `src-tauri/target/release/flowforge-ai.exe`, 9,034,752 bytes at this checkpoint. Its worker resource folder exists at `src-tauri/target/release/worker`.
- Cargo.lock and package-lock.json are ready to commit with the source. All dependency downloads/caches are already available locally.

Exact stopping point and next work:

1. **Installer bundling returned failure.** Tauri compiled the production executable, downloaded/hash-verified NSIS tools and ran makensis, but ended with Windows error 32: file in use. We launched the production app while bundling was still running; this is the likely file-lock cause, not yet confirmed by a successful retry. An installer file exists at `src-tauri/target/release/bundle/nsis/FlowForge AI_0.2.0_x64-setup.exe` (73,708,628 bytes), but **do not call it a verified successful installer** until a closed-app retry succeeds.
2. Close FlowForge before rebuilding. The app and Vite server were stopped for the pause. Run `npm run tauri -- build --config src-tauri/tauri.release.conf.json`. Existing packaged worker is current, so no PyInstaller rebuild is needed unless Python changes. Full repeatable packaging is `npm run package`.
3. Review adding the application license and `docs/THIRD_PARTY_NOTICES.md` to installer resources. The notices file exists but is not yet included in the release resource mapping. Actual FFmpeg DLL checked: 8.1.1, LGPL v3-or-later, GPL flag disabled. No complete public redistribution/license audit is claimed.
4. Verify actual desktop startup and local model readiness. The production app was launched once, but UI Automation inspection only saw WebView panes initially and returned no useful readiness evidence afterward. **No full native UI flow is verified yet.** A screenshot-only Edge check verified layout and the expected missing-desktop-bridge state; it did not exercise Rust IPC.
5. Test native microphone start/stop/cancel and shortcut behavior explicitly, then supported Notepad/browser fields, focus-change refusal and password-field refusal. No microphone was activated, and none of those application-specific acceptance checks has passed yet.
6. Verify installer install/uninstall in an isolated location, worker shutdown and crash cleanup. Builds are unsigned. No GitHub remote is configured; no remote CI run or public GitHub release occurred.

Development commands:

```powershell
Set-Location 'C:\Users\Mudit\OneDrive\Desktop\Python\FlowForge AI'
.\.venv\Scripts\python.exe -m pytest -q
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run desktop
# Close the running desktop app before packaging:
npm run tauri -- build --config src-tauri/tauri.release.conf.json
```

Temporary build investigation scripts/logs and screenshot are ignored under artifacts/. A temporary official sparse-index recovery script was needed for stalled registry freshness/DNS; normal locked downloads then worked. This helper is not an application dependency. Old generated `src-tauri/resources/worker` and `worker-build` folders can be cleaned after checking absolute paths; current source runtime output is `src-tauri/resources/flowforge-worker`.

## Previous continuation status - 2026-11-20 (historical)

The checkpoint above is historical. Work resumed when the owner said "continue".

- Closed-app release bundling succeeded, exit 0, producing a 70.29 MiB installer. License and third-party notices are now mapped into release resources.
- Silent per-user installation into ignored `artifacts/install-smoke` passed, including resource presence and notice/license hash checks. Silent uninstall passed and removed that directory and the uninstall registry entry. This was the development machine, not a clean-machine test.
- Installed native inspection exposed two actual defects: the first snapshot could run before `Shared` registration, leaving preferences unloaded and a persistent error; closing the main window left the hidden indicator and frozen worker alive.
- Source now registers runtime state before explicitly creating configured windows, and requests full application exit when the main window closes. Four Rust tests, strict Clippy, formatting, 47 Python tests and venv `pip check` pass after the fixes.
- The updated native debug build with embedded production assets and the frozen worker reaches `Ready. Your microphone is off.`, shows model `base`, enables recording and has no connection error. Normal close was also verified with a source worker. A forced termination of the earlier installed desktop killed its frozen worker. Startup removed a deliberately created stale session directory and preserved a non-session marker directory.
- Development-server launches showed a blank WebView despite localhost HTTP 200. The cause is not diagnosed. Embedded-assets startup works; do not claim that `npm run desktop` has a verified usable native UI yet.
- Cleanup of the non-session `installer-smoke-preserve` test-marker directory under app-local captures was rejected by automatic approval review as "blocked by policy". It remains; it contains a non-audio test marker, not a recording. Do not treat this as a microphone cleanup failure.
- Owner-guided microphone tests confirmed start/stop and recording-indicator appearance/disappearance, but the first automatic-language/default-input and subsequent English-only transcripts were inaccurate. The intended input was the Nirvana headset; Windows default was the Realtek array. Explicit headset selection alone did not resolve recognition. See C8 in CHALLENGES for the subsequent same-recording diagnostics and limited evidence for terminology hints.
- Final release bundling passed with the lifecycle fixes: installer 73,707,746 bytes, SHA-256 `7D96533F3C569269E3679070F315FED017F6C9146757AA105A15DAB361853933`; release executable 9,036,288 bytes. Installed native readiness, initial snapshot/preferences, single-instance protection, normal close/frozen-worker exit, forced idle desktop termination/frozen-worker exit, and uninstall/preferences preservation all passed on this development machine. The isolated installation has been removed.
- The current release executable is open (PID in `artifacts/app.pid`) using English/base/INT8/Nirvana headset/verbatim, VAD enabled and local vocabulary `Mudit, FlowForge AI`. No model download or LLM was added. A guided native test recognized both names and the remainder of the requested sentence; its leading word was absent. The cause of that omission is not established. Completion left zero capture directories. Do not claim broad accuracy, capture-start timing diagnosis or 100% recognition.

Next steps:

1. The owner-guided native cancellation check passed: cancelled status, microphone off, enabled Start, disabled Copy, empty preview and zero capture directories. Inference-time cancellation and recording-in-progress shutdown still need separate evidence. Same-recording recognition comparisons remain limited to the observed clips; no broad accuracy claim is justified.
2. Target-app checks are now in progress. **Automatic insertion is temporarily ON** via the real Preferences/Save flow and must be restored OFF after testing. A dedicated empty `artifacts/FlowForge-insertion-check.txt` is open in Notepad. The pending owner prompt asks for hold Ctrl+Alt+Space, wait for timer, dictate the supplied non-private phrase, release all modifiers and keep Notepad focused through insertion. Collect the result, then test browser insertion and focus/password refusal. Use ignored `artifacts/set-auto-insert.ps1 -Mode off` for the actual UI restore. Do not start recording automatically. Keep broad Hindi/Hinglish quality, LLM processing and fine-tuning gates separate.
3. The installer rebuild and lifecycle smoke checks are complete for the current source; do not redo them unless source/resource changes justify another build. No worker rebuild was needed because Python application source is unchanged. Only idle worker shutdown/crash checks passed; recording-in-progress shutdown still needs separate evidence.
4. Local checks are in ignored `artifacts/install-check.ps1`, `native-release-check.ps1`, `inspect-native.ps1`, `native-status.ps1`, `read-native-result.ps1` and `uninstall-check.ps1`. UI inspection returns transient output and must not persist transcripts. The diagnostic now explicitly imports the checkout because an older non-editable package copy exists in the venv; see C9. Its attempted no-dependency editable relink failed before installation because local build tooling lacked `bdist_wheel`; runtime dependency checks still pass.
5. Diagnose development WebView navigation separately. Clean-machine validation, exact-build redistribution audit, signing, remote CI and public publication remain unverified.

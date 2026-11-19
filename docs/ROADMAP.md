# Roadmap

1. **Speech foundation:** CLI, WAV, offline ASR, reference evaluation and model reuse are implemented and tested. Physical microphone verification remains separate.
2. **Desktop:** Tauri/React, CPAL capture, shortcuts, worker supervision, preview, cancellation, atomic preferences and guarded insertion/manual copy are implemented. Build/runtime and target-app checks require observed evidence.
3. **Correction:** original recovery, optional whitespace/filler/paragraph cleanup and terminology hints are implemented. No LLM without measured improvement.
4. **Streaming investigation:** optional bounded rolling preview and Silero VAD are implemented; real chunk comparisons ran. True incremental decoding and stable cumulative partials are not claimed.
5. **Research:** same-clip model/chunk/VAD experiments, WER/CER, latency and process peak memory measurements ran. Broader licensed multilingual data and justified fine-tuning remain future work.
6. **Release:** packaging scripts, supervision, single-instance protection, cleanup and Windows CI are implemented. Installer validation, clean-machine testing, signing and publication need separate evidence.

The owner subsequently authorized the remaining milestones together. Desktop, cleanup, bounded preview, experiments and packaging are now being delivered. Code availability is separate from acceptance evidence; see CHALLENGES for actual results.

Remaining gates: physical desktop microphone speech test; Notepad/browser insertion and focus-change refusal; broader consented Hindi/Hinglish evaluation; actual clean-machine installer, signing and public release checks. No LLM or fine-tuning is justified without measured benefit and suitable data. Rolling preview does not claim incremental decoding, stabilized cumulative text or automatic endpointing.

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

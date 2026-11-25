# Third-party components and redistribution

FlowForge application source is MIT licensed. That does not relicense bundled third-party code, native libraries or model weights. Runtime package metadata and available license files are retained in the packaged worker's `_internal/*dist-info` directories.

Principal components: faster-whisper and CTranslate2 (MIT), NumPy (BSD), PyAV (BSD source), ONNX Runtime (MIT), Hugging Face Hub/tokenizers (Apache-2.0), Python (PSF), Tauri/React and their transitive dependencies. Refer to the actual versions and provided upstream license texts; this list is not a complete dependency-license audit.

This software uses FFmpeg libraries through PyAV. The installed PyAV 17.1.0 wheel reports FFmpeg 8.1.1; its loaded avutil DLL reports **LGPL version 3 or later**, with `--enable-gpl` absent from its configuration. This was checked on the actual installed Windows wheel, rather than inferred from the PyAV project's source license or an unrelated wheel. Other wheels/builds may differ.

- FFmpeg project and redistribution guidance: https://ffmpeg.org/legal.html
- FFmpeg source releases: https://ffmpeg.org/releases/
- PyAV source and wheel build system: https://github.com/PyAV-Org/PyAV and https://github.com/PyAV-Org/pyav-ffmpeg
- faster-whisper: https://github.com/SYSTRAN/faster-whisper
- CTranslate2: https://github.com/OpenNMT/CTranslate2
- ONNX Runtime: https://github.com/microsoft/onnxruntime

Before a public binary release, complete the dependency inventory and corresponding-source/notice obligations for the exact bundled native builds and transitive libraries. Do not label the entire installer “MIT only” or claim that packaging success establishes redistribution compliance. Public distribution, signing and clean-machine verification remain release gates. No public release was published by this local build.

Models are not bundled. The desktop permits multilingual tiny/base/small from faster-whisper's identified Systran Hugging Face repositories. Review each model card and upstream Whisper license before redistributing weights. Custom datasets require consent or suitable licenses; the local JFK evaluation audio is not included in this application package.

## Supplementary license evidence (2026-11-25)

The existing frozen worker inventory contains 13 distribution metadata directories and 137 executable/native/model files. This does not enumerate every dependency. ONNX Runtime's license and third-party notices are present inside its package directory; they must not be inferred missing solely from its dist-info directory. The CTranslate2 license text was absent from its checked package. The first inventory incorrectly missed tqdm's existing British-spelled `licenses/LICENCE`; the matcher now recognizes that spelling. The installer includes supplementary pinned upstream texts and a full MPL text under `licenses/`:

- `CTranslate2-4.9.0-LICENSE.txt`: https://github.com/OpenNMT/CTranslate2/blob/v4.9.0/LICENSE, SHA-256 `54aa79d9fe3c09e67a16dcd95b9e88676405a6ec174efda31036983cf7672ecb`.
- `tqdm-4.70.1-LICENCE.txt`: https://github.com/tqdm/tqdm/blob/v4.70.1/LICENCE, SHA-256 `fcff87c3a47ce8028a8512aa182d4fcf0ad1c90544ee75cf9b343684cac194de`.
- `MPL-2.0.txt`: https://www.mozilla.org/media/MPL/2.0/index.txt, SHA-256 `3f3d9e0024b1921b067d6f7f88deb4a60cbe7a78e76c64e3f1d7fc3b779b9d04`.

These versioned texts cover identified gaps in this build's notices. Refresh the inventory and verify applicability when changing dependencies. Exact FFmpeg corresponding source/build settings and remaining native/transitive obligations still require review before binary publication. See https://ffmpeg.org/legal.html. No audit completion is claimed.

Run `python scripts/inventory-worker.py --worker src-tauri/resources/flowforge-worker --output reports/worker-inventory.json` to create a new, non-overwriting local report. Reports are ignored by Git.

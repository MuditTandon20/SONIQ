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

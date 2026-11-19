# Roadmap

1. **Current: speech foundation.** CLI capture, WAV output, local ASR, configuration comparisons, reference evaluation and meaningful tests. Complete interactive microphone verification before declaring all hardware acceptance criteria satisfied.
2. **Next: Windows desktop MVP.** Only after foundation verification: Tauri, small React UI, supervised Python IPC, recording shortcut, cancellation, preview and safe focus-aware insertion. Verify Notepad and a browser text field. Offer manual copy when insertion cannot be trusted.
3. **Contextual correction.** Preserve original text; evaluate deterministic cleanup before any LLM.
4. **Streaming.** Measure chunk sizes, VAD, stabilization and bounded buffering only after desktop behavior is reliable.
5. **Speech research.** Fixed licensed evaluation sets, multilingual/code-switching comparisons and measured resource use; fine-tune only for a demonstrated need.
6. **Release.** Windows installer, packaging, failure recovery, privacy review and CI after repeatable checks exist.

No later milestone is implemented or scaffolded now.

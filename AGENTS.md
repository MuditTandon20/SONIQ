# Engineering standards

The owner paused work on 2026-11-20 and subsequently said “continue” to resume. Resume from the current continuation status in docs/ROADMAP.md; do not restart implementation or redo completed dependency setup. No microphone recording had been activated at the original pause; subsequent explicit owner-guided microphone checks are recorded in docs/CHALLENGES.md.

The owner has now authorized the remaining milestones together. Deliver the Windows desktop dictation application incrementally, verifying each boundary before expanding it. Preserve the working Python CLI. Do not claim untested hardware, ML quality, or release checks are complete. Optional LLM processing and fine-tuning still require evidence of benefit.

Inspect existing work before editing. Prefer focused functions and standard-library solutions. Justify each dependency in docs/DECISIONS.md. No speculative interfaces, scaffolding, or fake ASR. Keep model initialization outside per-file transcription.

Microphone activation must be explicit and visible. Cancellation must close resources and discard partial recordings. Do not upload audio, execute transcripts, or store transcript history. Ignore recordings, reports and model weights in Git. Model downloads require an explicit CLI flag.

Run `.venv\Scripts\python.exe -m pytest -q` and `python -m pip check` in the venv after relevant changes. Separate real audio decoding tests, mocked hardware tests, real model integration checks, and interactive microphone checks. Never invent metrics or claim untested hardware works.

Keep README and docs synchronized. Record only actual challenges, confirmed causes, and observed outcomes. Explain major concepts using current code. Keep commits focused; do not commit generated files or private data. Preserve user changes.

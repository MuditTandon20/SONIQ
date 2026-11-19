# Engineering standards

Build only the assigned milestone. Milestone 1 is a Python CLI; do not add desktop, web, database, streaming, LLM, or cloud infrastructure. Stop before Milestone 2.

Inspect existing work before editing. Prefer focused functions and standard-library solutions. Justify each dependency in docs/DECISIONS.md. No speculative interfaces, scaffolding, or fake ASR. Keep model initialization outside per-file transcription.

Microphone activation must be explicit and visible. Cancellation must close resources and discard partial recordings. Do not upload audio, execute transcripts, or store transcript history. Ignore recordings, reports and model weights in Git. Model downloads require an explicit CLI flag.

Run `.venv\Scripts\python.exe -m pytest -q` and `python -m pip check` in the venv after relevant changes. Separate real audio decoding tests, mocked hardware tests, real model integration checks, and interactive microphone checks. Never invent metrics or claim untested hardware works.

Keep README and docs synchronized. Record only actual challenges, confirmed causes, and observed outcomes. Explain major concepts using current code. Keep commits focused; do not commit generated files or private data. Preserve user changes.

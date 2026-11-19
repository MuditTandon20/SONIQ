"""One model per session, with complete generator timing."""

from time import perf_counter
from pathlib import Path

import numpy as np

from .audio import load_audio


class Transcriber:
    def __init__(self, model="base", compute_type="int8", download=False):
        from faster_whisper import WhisperModel
        from faster_whisper.utils import download_model

        started = perf_counter()
        model_path = Path(model)
        if not model_path.is_dir():
            try:
                model_path = Path(download_model(model, local_files_only=not download))
            except Exception as exc:
                raise RuntimeError(f"Cannot load model {model!r}. Use --download once to fetch "
                                   f"trusted weights, or supply a complete local directory: {exc}") from exc
        # Upstream otherwise falls back to downloading a tokenizer even for a
        # local model. Require complete weights to keep offline mode offline.
        if not (model_path / "tokenizer.json").is_file():
            raise ValueError(f"Model directory lacks tokenizer.json: {model_path}. Supply a complete model.")
        self.model = WhisperModel(str(model_path), device="cpu", compute_type=compute_type,
                                  local_files_only=True)
        self.load_seconds = perf_counter() - started

    def transcribe(self, path: Path, language: str | None = None, beam_size: int = 5) -> dict:
        if beam_size < 1:
            raise ValueError("Beam size must be positive.")
        started = perf_counter()
        audio = load_audio(path)
        decode_seconds = perf_counter() - started
        duration = len(audio) / 16000
        started = perf_counter()
        text = ""
        detected = language
        # Exact digital silence is deterministic; quiet speech is not discarded.
        if np.any(audio):
            segments, info = self.model.transcribe(audio, language=language, beam_size=beam_size,
                                                    vad_filter=False, task="transcribe")
            text = "".join(segment.text for segment in segments).strip()
            detected = info.language
        elapsed = perf_counter() - started
        return {"audio": str(path), "text": text, "language": detected,
                "audio_seconds": duration, "decode_seconds": decode_seconds,
                "inference_seconds": elapsed, "real_time_factor": elapsed / duration}

"""Reference-based, reproducible corpus evaluation."""

import json
from pathlib import Path
import unicodedata

import jiwer
import numpy as np

from .asr import Transcriber


def normalize(text: str) -> str:
    # Preserve Indic vowel marks; replace punctuation with boundaries, not deletion.
    text = unicodedata.normalize("NFC", text).casefold()
    return " ".join("".join(" " if unicodedata.category(c).startswith("P") else c
                            for c in text).split())


def read_manifest(path: Path) -> list[dict]:
    entries = []
    with path.open(encoding="utf-8-sig") as source:
        for number, line in enumerate(source, 1):
            if not line.strip():
                continue
            try:
                entry = json.loads(line)
                if (not isinstance(entry, dict) or not isinstance(entry.get("audio"), str)
                        or not entry["audio"] or not isinstance(entry.get("reference"), str)):
                    raise ValueError("Expected audio path and reference string.")
                audio = (path.parent / entry["audio"]).resolve()
                if not audio.is_file():
                    raise ValueError(f"Missing audio: {audio}")
            except (ValueError, TypeError) as exc:
                raise ValueError(f"Manifest line {number}: {exc}") from exc
            entries.append({"audio": audio, "reference": entry["reference"]})
    if not entries:
        raise ValueError("Manifest contains no examples.")
    return entries


def evaluate(entries, model, compute_type="int8", language=None, repeats=1,
             beam_size=5, download=False) -> dict:
    if repeats < 1:
        raise ValueError("Repeats must be positive.")
    engine = Transcriber(model, compute_type, download)
    rows = []
    for _ in range(repeats):
        for entry in entries:
            rows.append(dict(engine.transcribe(entry["audio"], language, beam_size),
                             reference=entry["reference"]))
    references = [normalize(row["reference"]) for row in rows]
    hypotheses = [normalize(row["text"]) for row in rows]
    times = [row["inference_seconds"] for row in rows]
    return {"model": model, "device": "cpu", "compute_type": compute_type,
            "beam_size": beam_size, "language": language, "repeats": repeats,
            "model_load_seconds": engine.load_seconds,
            "normalization": "NFC, casefold, Unicode punctuation to spaces, collapse whitespace",
            "wer": jiwer.wer(references, hypotheses), "cer": jiwer.cer(references, hypotheses),
            "median_inference_seconds": float(np.median(times)),
            "p95_inference_seconds": float(np.percentile(times, 95)) if len(times) >= 20 else None,
            "real_time_factor": sum(times) / sum(row["audio_seconds"] for row in rows),
            "samples": rows}

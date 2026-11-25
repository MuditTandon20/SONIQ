from pathlib import Path

import numpy as np
import pytest

from soniq import experiments


def test_chunk_boundaries_include_final_short_chunk(monkeypatch):
    sizes = []

    class Engine:
        load_seconds = 0.1

        def __init__(self, model):
            pass

        def transcribe_samples(self, samples, language, **kwargs):
            sizes.append(len(samples))
            return {"text": "hello", "inference_seconds": 0.01}

    monkeypatch.setattr(experiments, "Transcriber", Engine)
    monkeypatch.setattr(experiments, "load_audio", lambda path: np.zeros(9 * 16000, np.float32))
    result = experiments.compare_chunks([{"audio": Path("clip.wav"), "reference": "hello hello hello"}],
                                         chunks=[4])
    assert sizes == [64000, 64000, 16000]
    assert result["configurations"][0]["wer"] == 0
    assert result["configurations"][0]["samples"][0]["audio_seconds"] == 9


@pytest.mark.parametrize("size", [0, -1, 31, float("nan")])
def test_invalid_chunk_sizes_rejected_before_loading(size):
    with pytest.raises(ValueError):
        experiments.compare_chunks([], chunks=[size])

import json
from pathlib import Path
from types import SimpleNamespace
import wave

import jiwer
import numpy as np
import pytest

from flowforge import audio, asr, evaluation
from flowforge.cli import main


def wav(path, rate=16000, channels=1, seconds=0.25, silent=False):
    signal = np.zeros(round(rate * seconds)) if silent else (
        np.sin(np.arange(round(rate * seconds)) * 2 * np.pi * 440 / rate) * 10000)
    samples = np.repeat(signal[:, None], channels, axis=1).astype("<i2")
    with wave.open(str(path), "wb") as output:
        output.setparams((channels, 2, rate, 0, "NONE", "not compressed"))
        output.writeframes(samples.tobytes())
    return path


@pytest.mark.parametrize("rate,channels", [(16000, 1), (48000, 2), (44100, 1)])
def test_real_decode_resample(tmp_path, rate, channels):
    data = audio.load_audio(wav(tmp_path / "signal.wav", rate, channels))
    assert data.dtype == np.float32
    assert data.ndim == 1
    assert abs(len(data) - 4000) <= 1
    assert np.max(np.abs(data)) > 0.1


@pytest.mark.parametrize("content", [None, b"", b"not audio"])
def test_invalid_audio(tmp_path, content):
    path = tmp_path / "invalid.wav"
    if content is not None:
        path.write_bytes(content)
    with pytest.raises(ValueError):
        audio.load_audio(path)


def test_empty_valid_wav(tmp_path):
    with pytest.raises(ValueError):
        audio.load_audio(wav(tmp_path / "empty.wav", seconds=0))


def test_model_reuse_and_lazy_segments(tmp_path, monkeypatch):
    calls = []

    class Model:
        def __init__(self, *args, **kwargs):
            calls.append(kwargs)

        def transcribe(self, data, **kwargs):
            def segments():
                yield SimpleNamespace(text=" hello")
                yield SimpleNamespace(text=" world")
            return segments(), SimpleNamespace(language="en")

    monkeypatch.setattr("faster_whisper.WhisperModel", Model)
    (tmp_path / "tokenizer.json").write_text("{}")
    monkeypatch.setattr("faster_whisper.utils.download_model", lambda *args, **kwargs: str(tmp_path))
    engine = asr.Transcriber()
    path = wav(tmp_path / "signal.wav")
    for _ in range(2):
        result = engine.transcribe(path)
        assert result["text"] == "hello world"
        assert result["audio_seconds"] == 0.25
        assert result["real_time_factor"] == result["inference_seconds"] / 0.25
    assert len(calls) == 1
    assert calls[0]["local_files_only"] is True


def test_incomplete_local_model_never_fetches_tokenizer(tmp_path):
    with pytest.raises(ValueError, match="tokenizer.json"):
        asr.Transcriber(str(tmp_path))


def test_exact_silence_skips_decoder(tmp_path):
    engine = object.__new__(asr.Transcriber)
    engine.model = SimpleNamespace()  # No transcribe method: a call would fail.
    assert engine.transcribe(wav(tmp_path / "silence.wav", silent=True))["text"] == ""


def test_normalization_preserves_hindi():
    assert evaluation.normalize("  HELLO, दुनिया! ") == "hello दुनिया"
    assert evaluation.normalize("हिंदी") == "हिंदी"


def test_corpus_metrics_and_model_reuse(tmp_path, monkeypatch):
    created = []

    class Engine:
        load_seconds = 1.0

        def __init__(self, *args):
            created.append(args)

        def transcribe(self, path, *args):
            return dict(audio=str(path), text="hello world", audio_seconds=2,
                        inference_seconds=0.5)

    monkeypatch.setattr(evaluation, "Transcriber", Engine)
    result = evaluation.evaluate([dict(audio=Path("x"), reference="Hello there!")],
                                 "tiny", repeats=2)
    assert result["wer"] == 0.5
    assert result["cer"] == jiwer.cer(["hello there"] * 2, ["hello world"] * 2)
    assert result["real_time_factor"] == 0.25
    assert result["p95_inference_seconds"] is None
    assert len(created) == 1


def test_manifest_relative_paths_and_empty_reference(tmp_path):
    wav(tmp_path / "a.wav")
    manifest = tmp_path / "examples.jsonl"
    manifest.write_text(json.dumps(dict(audio="a.wav", reference="")), encoding="utf-8")
    assert evaluation.read_manifest(manifest)[0]["audio"] == (tmp_path / "a.wav").resolve()


@pytest.mark.parametrize("line", ["", "{", "[]", '{"audio": "missing.wav", "reference": "hello"}',
                                  '{"audio": 12, "reference": "hello"}'])
def test_invalid_manifest(tmp_path, line):
    manifest = tmp_path / "bad.jsonl"
    manifest.write_text(line, encoding="utf-8")
    with pytest.raises(ValueError):
        evaluation.read_manifest(manifest)


class FakeStream:
    overflow = False
    cancel = False

    def __init__(self, **kwargs):
        pass

    def __enter__(self):
        return self

    def __exit__(self, *args):
        pass

    def read(self, frames):
        if self.cancel:
            raise KeyboardInterrupt
        return bytes(frames * 2), self.overflow


@pytest.fixture
def fake_mic(monkeypatch):
    monkeypatch.setattr("sounddevice.query_devices", lambda *args: dict(default_samplerate=16000))
    monkeypatch.setattr("sounddevice.check_input_settings", lambda **kwargs: None)
    monkeypatch.setattr("sounddevice.RawInputStream", FakeStream)


def test_record_valid_wav_and_no_overwrite(tmp_path, fake_mic):
    path = tmp_path / "capture.wav"
    audio.record(path, 0.1)
    with wave.open(str(path)) as captured:
        assert captured.getnframes() == 1600
        assert captured.getnchannels() == 1
        assert captured.getsampwidth() == 2
    with pytest.raises(ValueError, match="overwrite"):
        audio.record(path, 0.1)


@pytest.mark.parametrize("failure", ["overflow", "cancel"])
def test_record_discards_failure(tmp_path, fake_mic, monkeypatch, failure):
    monkeypatch.setattr(FakeStream, failure, True)
    with pytest.raises((RuntimeError, KeyboardInterrupt)):
        audio.record(tmp_path / "capture.wav", 0.1)
    assert list(tmp_path.iterdir()) == []


@pytest.mark.parametrize("seconds", [0, -1, 601, float("nan"), float("inf")])
def test_invalid_duration(tmp_path, seconds):
    with pytest.raises(ValueError):
        audio.record(tmp_path / "x.wav", seconds)


def test_cli_missing_audio(capsys):
    assert main(["transcribe", "missing.wav"]) == 1
    assert "does not exist" in capsys.readouterr().err


def test_cli_missing_mic(monkeypatch, capsys):
    monkeypatch.setattr("flowforge.cli.microphones", lambda: [])
    assert main(["devices"]) == 1
    assert "No microphone" in capsys.readouterr().err


def test_device_listing_accepts_native_index(monkeypatch):
    monkeypatch.setattr("sounddevice.query_devices", lambda: [
        dict(index=0, name="mic", max_input_channels=1),
        dict(index=1, name="speaker", max_input_channels=0)])
    assert audio.microphones() == [dict(index=0, name="mic", max_input_channels=1)]


def test_cli_cancel(monkeypatch):
    def cancel(*args):
        raise KeyboardInterrupt
    monkeypatch.setattr("builtins.input", cancel)
    assert main(["record", "unused.wav"]) == 130

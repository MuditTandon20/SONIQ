"""Explicit microphone capture and shared audio decoding."""

from pathlib import Path
import math
import os
import tempfile
import wave

import numpy as np


def microphones():
    import sounddevice as sd

    return [dict(device, index=i) for i, device in enumerate(sd.query_devices())
            if device["max_input_channels"] > 0]


def record(path: Path, seconds: float, device: int | None = None) -> dict:
    """Write bounded PCM capture atomically; cancellation leaves no partial WAV."""
    import sounddevice as sd

    if not math.isfinite(seconds) or not 0 < seconds <= 600:
        raise ValueError("Recording duration must be greater than 0 and at most 600 seconds.")
    if path.exists():
        raise ValueError(f"Refusing to overwrite {path}.")
    info = sd.query_devices(device, "input")
    rate = round(info["default_samplerate"])
    if round(rate * seconds) < 1:
        raise ValueError("Recording duration is shorter than one audio sample.")
    sd.check_input_settings(device=device, channels=1, dtype="int16", samplerate=rate)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(suffix=".wav", dir=path.parent)
    os.close(fd)
    try:
        with wave.open(temporary, "wb") as output:
            output.setparams((1, 2, rate, 0, "NONE", "not compressed"))
            with sd.RawInputStream(device=device, samplerate=rate, channels=1,
                                   dtype="int16") as stream:
                print(f"RECORDING: {seconds:g}s. Ctrl+C cancels and discards audio.", flush=True)
                remaining = round(rate * seconds)
                while remaining:
                    frames = min(remaining, max(1, rate // 20))
                    data, overflowed = stream.read(frames)
                    if overflowed:
                        raise RuntimeError("Microphone input overflowed; recording discarded. Close busy apps and retry.")
                    output.writeframesraw(bytes(data))
                    remaining -= frames
        # Same-directory hard link publishes a complete file without overwriting.
        os.link(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)
    return {"path": str(path), "sample_rate": rate, "seconds": round(rate * seconds) / rate}


def load_audio(path: Path) -> np.ndarray:
    """PyAV (via faster-whisper) decodes, mixes to mono, and resamples to 16 kHz."""
    from faster_whisper.audio import decode_audio

    if not path.is_file():
        raise ValueError(f"Audio file does not exist: {path}")
    if path.stat().st_size == 0:
        raise ValueError(f"Audio file is empty: {path}")
    try:
        audio = decode_audio(str(path), sampling_rate=16000)
    except (ValueError, RuntimeError, OSError) as exc:
        raise ValueError(f"Cannot decode audio {path}: {exc}") from exc
    if audio.size == 0 or not np.isfinite(audio).all():
        raise ValueError(f"Audio contains no valid samples: {path}")
    return audio

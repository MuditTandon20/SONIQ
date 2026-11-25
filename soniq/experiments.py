"""Measure the accuracy/latency trade-off of independent ASR chunks."""

import ctypes
import os
import platform
from time import perf_counter

import jiwer
import numpy as np

from .asr import Transcriber
from .audio import load_audio
from .evaluation import normalize


def peak_memory_bytes():
    if os.name != "nt":
        return None

    class Counters(ctypes.Structure):
        _fields_ = [("cb", ctypes.c_ulong), ("page_faults", ctypes.c_ulong)] + [
            (name, ctypes.c_size_t) for name in ("peak_working_set", "working_set", "quota_peak_paged",
            "quota_paged", "quota_peak_nonpaged", "quota_nonpaged", "pagefile", "peak_pagefile")]

    counters = Counters()
    counters.cb = ctypes.sizeof(counters)
    process = ctypes.windll.kernel32.GetCurrentProcess
    process.restype = ctypes.c_void_p
    query = ctypes.windll.psapi.GetProcessMemoryInfo
    query.argtypes = [ctypes.c_void_p, ctypes.POINTER(Counters), ctypes.c_ulong]
    if not query(process(), ctypes.byref(counters), counters.cb):
        return None
    return counters.peak_working_set


def compare_chunks(entries, model="tiny", chunks=(4, 8, 12), language=None, vad=False):
    if any(not np.isfinite(size) or not 1 <= size <= 30 for size in chunks):
        raise ValueError("Chunk sizes must be between 1 and 30 seconds.")
    engine = Transcriber(model)
    reports = []
    for size in chunks:
        rows = []
        for entry in entries:
            waveform = load_audio(entry["audio"])
            chunk_samples = round(size * 16000)
            started = perf_counter()
            segments = [engine.transcribe_samples(waveform[offset:offset + chunk_samples],
                        language, beam_size=1, vad=vad)
                        for offset in range(0, len(waveform), chunk_samples)]
            elapsed = perf_counter() - started
            rows.append({"audio": str(entry["audio"]), "reference": entry["reference"],
                         "text": " ".join(row["text"] for row in segments).strip(),
                         "audio_seconds": len(waveform) / 16000, "inference_seconds": elapsed,
                         "chunk_inference_seconds": [row["inference_seconds"] for row in segments]})
        references = [normalize(row["reference"]) for row in rows]
        hypotheses = [normalize(row["text"]) for row in rows]
        timings = [time for row in rows for time in row["chunk_inference_seconds"]]
        reports.append({"chunk_seconds": size, "wer": jiwer.wer(references, hypotheses),
                        "cer": jiwer.cer(references, hypotheses),
                        "median_chunk_inference_seconds": float(np.median(timings)),
                        "real_time_factor": sum(row["inference_seconds"] for row in rows) /
                                            sum(row["audio_seconds"] for row in rows),
                        "samples": rows})
    return {"model": model, "compute_type": "int8", "beam_size": 1, "vad": vad,
            "language": language, "platform": platform.platform(), "python": platform.python_version(),
            "model_load_seconds": engine.load_seconds, "peak_process_memory_bytes": peak_memory_bytes(),
            "method": "Independent non-overlapping chunks, no stabilization or warmup. "
                      "Memory is process peak across all configurations; not per-model memory.",
            "configurations": reports}

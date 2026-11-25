"""Small command-line entry point. No microphone activation on import/startup."""

import argparse
from importlib.metadata import version
import json
import os
from pathlib import Path
import platform
import sys

from .audio import microphones, record
from .asr import Transcriber
from .evaluation import evaluate, read_manifest


def main(argv=None) -> int:
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description="SONIQ - local speech recognition")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("devices", help="List microphones without recording")
    capture = commands.add_parser("record", help="Explicit timed microphone capture")
    capture.add_argument("output", type=Path)
    capture.add_argument("--seconds", type=float, default=5)
    capture.add_argument("--device", type=int)
    transcription = commands.add_parser("transcribe", help="Transcribe one or more local audio files")
    transcription.add_argument("audio", type=Path, nargs="+")
    benchmark = commands.add_parser("evaluate", help="Compare configurations on reference JSONL")
    benchmark.add_argument("manifest", type=Path)
    benchmark.add_argument("--repeats", type=int, default=1)
    benchmark.add_argument("--report", type=Path, help="Opt-in storage of transcripts and references")
    experiment = commands.add_parser("experiment", help="Measure chunk-size accuracy and latency")
    experiment.add_argument("manifest", type=Path)
    experiment.add_argument("--model", default="tiny")
    experiment.add_argument("--chunks", type=float, nargs="+", default=[4, 8, 12])
    experiment.add_argument("--language")
    experiment.add_argument("--vad", action="store_true")
    experiment.add_argument("--report", type=Path)
    for command in (transcription, benchmark):
        command.add_argument("--model", nargs="+" if command is benchmark else None,
                             default=["base"] if command is benchmark else "base")
        command.add_argument("--compute-type", choices=["int8", "float32"], default="int8")
        command.add_argument("--language", help="Language code, e.g. en or hi; default auto-detect")
        command.add_argument("--beam-size", type=int, default=5)
        command.add_argument("--download", action="store_true", help="Allow model download from Hugging Face")
    args = parser.parse_args(argv)
    try:
        if args.command == "devices":
            devices = microphones()
            if not devices:
                raise RuntimeError("No microphone devices found. Check Windows microphone permissions and connection.")
            result = devices
        elif args.command == "record":
            input("Press Enter to start microphone recording (Ctrl+C cancels): ")
            result = record(args.output, args.seconds, args.device)
            print("Recording stopped.", flush=True)
        elif args.command == "experiment":
            from .experiments import compare_chunks
            result = compare_chunks(read_manifest(args.manifest), args.model, args.chunks,
                                    args.language, args.vad)
            if args.report:
                args.report.parent.mkdir(parents=True, exist_ok=True)
                with args.report.open("x", encoding="utf-8") as output:
                    json.dump(result, output, ensure_ascii=False, indent=2)
        else:
            if args.beam_size < 1:
                raise ValueError("Beam size must be positive.")
            if args.command == "transcribe":
                for path in args.audio:
                    if not path.is_file():
                        raise ValueError(f"Audio file does not exist: {path}")
                engine = Transcriber(args.model, args.compute_type, args.download)
                result = {"model_load_seconds": engine.load_seconds,
                          "results": [engine.transcribe(path, args.language, args.beam_size)
                                      for path in args.audio]}
            else:
                if args.repeats < 1:
                    raise ValueError("Repeats must be positive.")
                entries = read_manifest(args.manifest)
                result = {"environment": {"platform": platform.platform(),
                          "python": platform.python_version(), "processor": platform.processor(),
                          "logical_cpus": os.cpu_count(),
                          "packages": {name: version(name) for name in
                                       ("faster-whisper", "ctranslate2", "sounddevice", "jiwer", "numpy")}},
                          "configurations": [evaluate(entries, model, args.compute_type,
                              args.language, args.repeats, args.beam_size, args.download)
                              for model in args.model]}
                if args.report:
                    args.report.parent.mkdir(parents=True, exist_ok=True)
                    with args.report.open("x", encoding="utf-8") as output:
                        json.dump(result, output, ensure_ascii=False, indent=2)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0
    except (KeyboardInterrupt, EOFError):
        print("Cancelled.", file=sys.stderr)
        return 130
    except Exception as exc:
        # Last CLI boundary: native PortAudio, PyAV, CT2 and download errors have
        # unrelated exception hierarchies. Lower layers preserve their exceptions.
        print(f"Error: {exc}", file=sys.stderr)
        return 1

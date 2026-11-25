"""Supervised local NDJSON worker. stdout is reserved for protocol messages."""

import json
from pathlib import Path
import sys

from .asr import Transcriber
from .text import clean_text


def serve(source, destination):
    engine = None
    configuration = None
    for line in source:
        request = {}
        try:
            if len(line) > 65536:
                raise ValueError("Command exceeds 64 KiB.")
            request = json.loads(line)
            if not isinstance(request, dict):
                raise ValueError("Command must be an object.")
            command = request.get("command")
            if command == "shutdown":
                break
            if command == "load":
                model = request.get("model", "base")
                compute = request.get("compute_type", "int8")
                if not isinstance(model, str) or compute not in ("int8", "float32"):
                    raise ValueError("Invalid model configuration.")
                key = (model, compute)
                if key != configuration:
                    engine = None
                    configuration = None
                    engine = Transcriber(model, compute, bool(request.get("download", False)))
                    configuration = key
                result = {"event": "ready", "model": model, "load_seconds": engine.load_seconds}
            elif command in ("transcribe", "preview"):
                if engine is None:
                    raise ValueError("Load a model before transcription.")
                if not isinstance(request.get("audio"), str):
                    raise ValueError("Audio path must be a string.")
                result = engine.transcribe(Path(request["audio"]), request.get("language"),
                                           vad=bool(request.get("vad", False)),
                                           hotwords=request.get("hotwords"),
                                           beam_size=1 if command == "preview" else 5)
                result["raw_text"] = result["text"]
                result["text"] = clean_text(result["raw_text"], request.get("mode", "verbatim"),
                                            bool(request.get("remove_fillers", False)))
                result["event"] = "partial" if command == "preview" else "transcript"
            else:
                raise ValueError(f"Unknown command: {command!r}")
            result["id"] = request.get("id")
        except Exception as exc:
            result = {"event": "error", "id": request.get("id") if isinstance(request, dict) else None,
                      "message": str(exc)}
        destination.write(json.dumps(result, ensure_ascii=False) + "\n")
        destination.flush()


if __name__ == "__main__":
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    serve(sys.stdin, sys.stdout)

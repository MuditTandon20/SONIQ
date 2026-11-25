from io import StringIO
import json

import pytest

from soniq import worker
from soniq.text import clean_text


def test_worker_reuses_model_and_recovers_after_bad_commands(monkeypatch):
    instances = []

    class Engine:
        load_seconds = 0.2

        def __init__(self, *args):
            instances.append(args)

        def transcribe(self, path, language, **kwargs):
            return {"text": "  Hello  world. Next thought.  ", "inference_seconds": 0.1}

    monkeypatch.setattr(worker, "Transcriber", Engine)
    requests = [
        {"id": 1, "command": "load", "model": "tiny"},
        {"id": 2, "command": "load", "model": "tiny"},
        {"id": 3, "command": "transcribe", "audio": "x.wav", "mode": "paragraphs"},
        {"id": 4, "command": "invalid"},
        {"id": 5, "command": "preview", "audio": "x.wav"},
        {"command": "shutdown"},
        {"command": "load", "model": "base"},
    ]
    output = StringIO()
    worker.serve(StringIO("\n".join(json.dumps(r) for r in requests)), output)
    rows = [json.loads(line) for line in output.getvalue().splitlines()]
    assert len(instances) == 1
    assert len(rows) == 5
    assert rows[2]["raw_text"] == "  Hello  world. Next thought.  "
    assert rows[2]["text"] == "Hello world.\n\nNext thought."
    assert rows[3]["event"] == "error"
    assert rows[3]["id"] == 4
    assert rows[4]["event"] == "partial"


def test_worker_protocol_validation_and_unloaded_engine():
    output = StringIO()
    worker.serve(StringIO('[]\n{\n{"command":"transcribe","audio":"x"}\n'), output)
    rows = [json.loads(line) for line in output.getvalue().splitlines()]
    assert all(row["event"] == "error" for row in rows)
    assert "Load a model" in rows[2]["message"]


@pytest.mark.parametrize("text", ["use foo_bar", "git --help", "return x/y", "At 12:30 on 2026-11-19",
                                   "https://example.com", "`a  b`", "$HOME", "x = 5"])
def test_cleanup_preserves_technical_and_numeric_transcripts(text):
    assert clean_text(text, "paragraphs", True) == text


def test_cleanup_opt_in_and_preserves_hindi_and_original():
    assert clean_text(" um, Hello  world. ") == " um, Hello  world. "
    assert clean_text("um, Hello  world.", "clean", True) == "Hello world."
    assert clean_text("नमस्ते दुनिया। अगला वाक्य।", "paragraphs") == "नमस्ते दुनिया।\n\nअगला वाक्य।"
    assert clean_text("umbrella hum yummy", "clean", True) == "umbrella hum yummy"


def test_unknown_cleanup_mode_rejected():
    with pytest.raises(ValueError):
        clean_text("hello", "unknown")

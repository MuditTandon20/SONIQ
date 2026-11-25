"""Inventory an existing frozen worker without starting it or reading audio.

This records build evidence, not a complete license or security audit.
"""

import argparse
from email.parser import Parser
import hashlib
import json
from pathlib import Path


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(block)
    return checksum.hexdigest()


def inventory(worker):
    executable = worker / "soniq-worker.exe"
    internal = worker / "_internal"
    if not executable.is_file() or not internal.is_dir():
        raise ValueError("Expected an existing onedir soniq-worker package.")
    packages = []
    for metadata in sorted(internal.glob("*.dist-info/METADATA")):
        message = Parser().parsestr(metadata.read_text(encoding="utf-8"))
        roots = [metadata.parent]
        top_level = metadata.parent / "top_level.txt"
        if top_level.is_file():
            roots.extend(internal / name for name in top_level.read_text(encoding="utf-8").splitlines()
                         if name.isidentifier() and (internal / name).is_dir())
        conventional = internal / message["Name"].replace("-", "_")
        if conventional.is_dir() and conventional not in roots:
            roots.append(conventional)
        licenses = [
            {"path": path.relative_to(worker).as_posix(), "sha256": digest(path)}
            for path in sorted({path for root in roots for path in root.rglob("*")})
            if path.is_file() and any(
                marker in path.name.lower() for marker in ("license", "licence", "copying", "notice")
            )
        ]
        packages.append({
            "name": message["Name"], "version": message["Version"],
            "license_expression": message["License-Expression"],
            "project_urls": message.get_all("Project-URL", []),
            "license_files": licenses,
        })
    binaries = [
        {"path": path.relative_to(worker).as_posix(), "bytes": path.stat().st_size,
         "sha256": digest(path)}
        for path in sorted(worker.rglob("*"))
        if path.is_file() and path.suffix.lower() in (".exe", ".dll", ".pyd", ".onnx")
    ]
    return {
        "scope": "Existing frozen files and bundled distribution metadata only; not a complete dependency, corresponding-source, license or security audit.",
        "packages": packages, "binaries": binaries,
        "packages_without_bundled_license_files": [
            item["name"] for item in packages if not item["license_files"]
        ],
        "remaining_review": [
            "Identify dependencies without bundled distribution metadata, including native DLL dependencies.",
            "Verify corresponding source, patches, build settings and notices for the exact FFmpeg/native binaries.",
            "Verify Python, PortAudio, OpenSSL, runtime redistributables and transitive component notices.",
        ],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = inventory(args.worker)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x", encoding="utf-8") as output:
        json.dump(report, output, indent=2, ensure_ascii=False)
        output.write("\n")
    print(json.dumps({"packages": len(report["packages"]), "binaries": len(report["binaries"]),
                      "packages_without_bundled_license_files": report["packages_without_bundled_license_files"]}))

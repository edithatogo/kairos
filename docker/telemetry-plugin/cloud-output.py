#!/usr/bin/env python3
"""Copy telemetry output to a cloud-style URI and write a SHA-256 sidecar.

The first implementation is intentionally offline-testable: file:// URIs are
copied locally, while s3://, gs://, and az:// URIs produce provider manifests
that provider-specific upload jobs can consume in cloud environments.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path
from urllib.parse import urlparse
from urllib.request import url2pathname


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_checksum(path: Path, digest: str) -> Path:
    sidecar = Path(f"{path}.sha256")
    sidecar.write_text(f"{digest}  {path.name}\n", encoding="utf-8")
    return sidecar


def copy_file_uri(source: Path, destination: str) -> Path:
    parsed = urlparse(destination)
    target_root = Path(url2pathname(parsed.path)) if parsed.scheme == "file" else Path(parsed.path)
    target_root.mkdir(parents=True, exist_ok=True)
    target = target_root / source.name
    shutil.copy2(source, target)
    return target


def write_provider_manifest(source: Path, destination: str, digest: str) -> Path:
    manifest = source.with_suffix(source.suffix + ".upload.json")
    parsed = urlparse(destination)
    manifest.write_text(
        json.dumps(
            {
                "schema": "kairo.ecs.telemetry-upload.v1",
                "source": str(source),
                "destination": destination,
                "provider": parsed.scheme,
                "checksum_sha256": digest,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, help="Arrow telemetry file")
    parser.add_argument("--destination", required=True, help="file://, s3://, gs://, or az:// URI")
    args = parser.parse_args()

    source = Path(args.input)
    if not source.is_file():
        raise SystemExit(f"input file does not exist: {source}")

    digest = sha256_file(source)
    parsed = urlparse(args.destination)
    if parsed.scheme in ("", "file"):
        target = copy_file_uri(source, args.destination)
        checksum = write_checksum(target, digest)
        print(json.dumps({"copied": str(target), "checksum": str(checksum), "sha256": digest}))
        return 0

    if parsed.scheme not in {"s3", "gs", "az"}:
        raise SystemExit(f"unsupported destination scheme: {parsed.scheme}")

    checksum = write_checksum(source, digest)
    manifest = write_provider_manifest(source, args.destination, digest)
    print(json.dumps({"manifest": str(manifest), "checksum": str(checksum), "sha256": digest}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

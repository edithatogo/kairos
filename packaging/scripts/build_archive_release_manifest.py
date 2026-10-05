#!/usr/bin/env python3
"""Create release subjects from verified actual archives, never source manifests."""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil


def build(source: Path, output: Path, source_commit: str) -> dict:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ValueError("expected full lowercase source SHA")
    if output.exists():
        raise ValueError("release evidence output must not exist")
    spec = importlib.util.spec_from_file_location("bundle_verifier", Path(__file__).with_name("build_package_archive_bundle.py"))
    verifier = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(verifier)
    verifier.verify(source)
    index_bytes = (source / "ARCHIVE-INDEX.json").read_bytes()
    index = json.loads(index_bytes)
    if index["source_commit"] != source_commit:
        raise ValueError("archive bundle source SHA differs")
    artifacts = []
    output.mkdir(parents=True)
    try:
        for row in index["artifacts"]:
            original = verifier.indexed_path(source, row["path"])
            relative = "archives/" + row["path"]
            target = output / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(original, target)
            if verifier.checksum(target) != row["sha256"] or target.stat().st_size != row["bytes"]:
                raise ValueError("copied archive differs from verified index")
            artifacts.append({"path": relative, "sha256": row["sha256"], "bytes": row["bytes"],
                "ecosystem": "csharp" if row["ecosystem"] == "nuget" else row["ecosystem"],
                "archive_ecosystem": row["ecosystem"], "kind": row["kind"]})
        # Detect input changes during acquisition/copy; no new build is implied.
        verifier.verify(source)
        if (source / "ARCHIVE-INDEX.json").read_bytes() != index_bytes:
            raise ValueError("archive index changed during manifest creation")
        artifacts.sort(key=lambda row: row["path"])
        manifest = {"schema_version": 1, "release_stage": "actual-package-archives",
            "source_commit": source_commit, "production_publish_enabled": False,
            "archive_index_sha256": hashlib.sha256(index_bytes).hexdigest(), "artifacts": artifacts}
        (output / "release-artifact-manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
        (output / "SHA256SUMS").write_text("".join(f"{row['sha256']}  {row['path']}\n" for row in artifacts))
        (output / "RELEASE.txt").write_text(f"Verified package archives from {source_commit}. Evidence preparation only; publication disabled.\n")
        return manifest
    except BaseException:
        shutil.rmtree(output)
        raise


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    args = parser.parse_args()
    result = build(args.input, args.output, args.source_commit)
    print(f"generated release subjects for {len(result['artifacts'])} actual archives")

if __name__ == "__main__":
    main()

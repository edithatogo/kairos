#!/usr/bin/env python3
"""Record the command and toolchain used for one package archive job."""

from __future__ import annotations

import argparse
import json
import platform
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ecosystem", required=True)
    parser.add_argument("--archive-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--command", required=True)
    parser.add_argument("--toolchain", required=True)
    parser.add_argument("--validation-notes", default="")
    args = parser.parse_args()
    if len(args.source_commit) != 40:
        parser.error("--source-commit must be the full 40-character Git SHA")
    if not args.archive_dir.is_dir():
        parser.error(f"archive directory does not exist: {args.archive_dir}")
    receipt = {
        "ecosystem": args.ecosystem,
        "source_commit": args.source_commit,
        "command": args.command,
        "toolchain": args.toolchain,
        "platform": platform.platform(),
        "exit_status": 0,
        "validation_notes": args.validation_notes,
    }
    path = args.archive_dir / "BUILD-INFO.json"
    path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(path)


if __name__ == "__main__":
    main()

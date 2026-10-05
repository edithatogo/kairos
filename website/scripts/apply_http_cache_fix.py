#!/usr/bin/env python3
"""Apply a verified local cache-source mitigation to npm's 4.3.0 package.

This is a local source mitigation, not an official patched upstream release.
It reuses the reviewed Track 45 adapter's source-fragment, tree-safety,
validation-before-write, atomic-replacement, and idempotence implementation.
"""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import stat
import sys


PACKAGE_VERSION = "4.3.0"
SOURCE_SHA256 = "ede1cc404a492fa348eb9d97a3007a0d72aa717bd22cd86a56bd0824c19729ca"
PR58_PATCHED_SHA256 = "7a23f143046560191aba075d4404821db051118d46d7419a2fd006154a1889eb"
PATCHED_SHA256 = "1c7d64faf562b93a3a989fe931aec6877b18c4f3e18b7822bb8647e1b3e2678e"
LEGACY_HELPER_SHA256 = "1745f11f6b2ae27c47ba00218970192ec0b0034d3d1467b524e411e2c3c9afa4"
FIXTURE_SHA256 = "d75e1e6a11587954da5e2f0e2b5c4b397a16d28cc2f7bdf64e9027fc2fe593ee"
FIXTURE_SRI = (
    "sha512-M5t5LlJpS1UHMjvwRQVdFHvPISGeLAxNcrWuJkeGh0KxsqCHZ1O3NXZU/"
    "8x7cD0BDcGW8kapxMKTvwlqrNkHkA=="
)


def _helper_path() -> Path:
    return Path(__file__).resolve().parents[2] / "scripts/bootstrap-node-tools/apply_http_cache_fix.py"


def _load_verified_helper() -> dict[str, object]:
    path = _helper_path()
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    fd = os.open(path, flags)
    try:
        if not stat.S_ISREG(os.fstat(fd).st_mode):
            raise ValueError("legacy patch helper must be a regular file")
        with os.fdopen(fd, "rb", closefd=False) as stream:
            source = stream.read()
    finally:
        os.close(fd)

    digest = hashlib.sha256(source).hexdigest()
    if digest != LEGACY_HELPER_SHA256:
        raise ValueError(f"legacy patch helper hash mismatch: {digest}")

    namespace: dict[str, object] = {
        "__name__": "_website69_verified_http_cache_helper_43",
        "__file__": str(path),
    }
    exec(compile(source, str(path), "exec"), namespace, namespace)
    namespace["PACKAGE_VERSION"] = PACKAGE_VERSION
    namespace["SOURCE_SHA256"] = SOURCE_SHA256
    namespace["PR58_PATCHED_SHA256"] = PR58_PATCHED_SHA256
    namespace["PATCHED_SHA256"] = PATCHED_SHA256
    return namespace


def pr58_source(source: bytes) -> bytes:
    return _load_verified_helper()["pr58_source"](source)  # type: ignore[operator]


def composed_source(source: bytes) -> bytes:
    return _load_verified_helper()["composed_source"](source)  # type: ignore[operator]


def patched_source(source: bytes) -> bytes:
    return _load_verified_helper()["patched_source"](source)  # type: ignore[operator]


def patch_tree(root: Path) -> list[tuple[Path, str]]:
    return _load_verified_helper()["patch_tree"](root)  # type: ignore[operator]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path, help="installed node_modules root to scan")
    args = parser.parse_args()
    try:
        for path, digest in patch_tree(args.root):
            print(f"{path}: {digest}")
        print(
            "LOCAL MITIGATION APPLIED; package identity remains "
            "http-cache-semantics@4.3.0 (not an official patched release)"
        )
        return 0
    except (OSError, ValueError) as exc:
        parser.exit(1, f"cache source mitigation failed: {exc}\n")


if __name__ == "__main__":
    raise SystemExit(main())

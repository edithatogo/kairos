#!/usr/bin/env python3
"""Apply a composed local http-cache-semantics source mitigation to an npm tree.

This composes upstream PR #58 and the stale-fallback fixes from PR #1; it is
not an official patched release. It keeps the registry package name and
version unchanged. The original, intermediate, and candidate index.js
SHA-256 values are pinned below; the upstream BSD-2-Clause license remains in
the package archive.
"""

# Original source-fragment license (BSD-2-Clause):
# Copyright 2016-2018 Kornel Lesiński
#
# Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:
#
# 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.
#
# 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import tempfile


UPSTREAM_COMMITS = (
    "14a8c2ad51740dc39bf3e8f1a11c845a5003f217",
    "101a9e9a5b9aba5750a74b8f659c5646e90a962f",
)

PACKAGE_NAME = "http-cache-semantics"
PACKAGE_VERSION = "4.2.0"
SOURCE_SHA256 = "01b7d66c854b2fe53ac05c98feb6e0d64722ab8898a778e2d2426a8b468d178f"
PR58_PATCHED_SHA256 = "fc7b3f0265b7a7d0fee83bafa47186a66495720d3179801c2be3083de6d0cf76"
PATCHED_SHA256 = "5942c6d3df40fce2151d8e409e7ad7e7c9c4a8ee09b7066072edf3a939fc589c"

# Exact source fragments from the hash-pinned release and upstream PR commits.
OLD_EVALUATE = b"""        // In all circumstances, a cache MUST NOT ignore the must-revalidate directive
        if (this._rescc['must-revalidate']) {
"""
NEW_EVALUATE = b"""        // Request directives cannot override restrictions on reusing the response.
        if (this._rescc['must-revalidate'] || this._requiresRevalidation()) {
"""
OLD_MAX_AGE_GUARD = b"""        if (!this.storable() || this._rescc['no-cache']) {
            return 0;
        }

        // Shared responses with cookies are cacheable according to the RFC, but IMHO it'd be unwise to do so by default
        // so this implementation requires explicit opt-in via public header
        if (
            this._isShared &&
            (this._resHeaders['set-cookie'] &&
                !this._rescc.public &&
                !this._rescc.immutable)
        ) {
            return 0;
        }
"""
NEW_REVALIDATION_HELPER = b"""    /**
     * Distinguishes reuse restrictions from ordinary expiration.
     * @returns {boolean} Whether this response must not be reused without validation.
     */
    _requiresRevalidation() {
        return !!(
            !this.storable() ||
            this._rescc['no-cache'] ||
            (this._isShared &&
                (this._rescc['proxy-revalidate'] ||
                    // Sharing responses with cookies requires an explicit opt-in.
                    (this._resHeaders['set-cookie'] &&
                        !this._rescc.public &&
                        !this._rescc.immutable)))
        );
    }

    /**
     * Possibly outdated value of applicable max-age (or heuristic equivalent) in seconds.
"""
OLD_MAX_AGE_DOC = b"""    /**
     * Possibly outdated value of applicable max-age (or heuristic equivalent) in seconds.
"""
NEW_MAX_AGE_GUARD = b"""        if (this._requiresRevalidation()) {
            return 0;
        }
"""
OLD_VARY_MAX_AGE_GUARD = b"""        if (this._resHeaders.vary === '*') {
            return 0;
        }

"""
OLD_NO_CACHE_REVALIDATION = b"""            this._rescc['no-cache'] ||
"""
NEW_NO_CACHE_REVALIDATION = b"""            this._rescc['no-cache'] ||
            this._resHeaders.vary === '*' ||
"""
OLD_PROXY_REVALIDATE = b"""            if (this._rescc['proxy-revalidate']) {
                return 0;
            }
"""
OLD_STALE_IF_ERROR = b"""        return this.maxAge() + toNumberOrZero(this._rescc['stale-if-error']) > this.age();
"""
NEW_STALE_IF_ERROR = b"""        return (
            !this._requiresRevalidation() &&
            !this._rescc['must-revalidate'] &&
            this.maxAge() + toNumberOrZero(this._rescc['stale-if-error']) > this.age()
        );
"""
OLD_STALE_WHILE_REVALIDATE = b"""        return swr > 0 && this.maxAge() + swr > this.age();
"""
NEW_STALE_WHILE_REVALIDATE = b"""        return (
            !this._requiresRevalidation() &&
            !this._rescc['must-revalidate'] &&
            swr > 0 && this.maxAge() + swr > this.age()
        );
"""
OLD_REVALIDATED_POLICY = b"""        if (this._useStaleIfError() && isErrorResponse(response)) {
"""
NEW_REVALIDATED_POLICY = b"""        if (
            this._requestMatches(request, true) &&
            this._useStaleIfError() &&
            isErrorResponse(response)
        ) {
"""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def replace_once(source: bytes, old: bytes, new: bytes, description: str) -> bytes:
    count = source.count(old)
    if count != 1:
        raise ValueError(f"expected exactly one {description} source fragment; found {count}")
    return source.replace(old, new, 1)


def pr58_source(source: bytes) -> bytes:
    result = replace_once(source, OLD_EVALUATE, NEW_EVALUATE, "request revalidation guard")
    result = replace_once(result, OLD_MAX_AGE_DOC, NEW_REVALIDATION_HELPER, "revalidation helper insertion")
    result = replace_once(result, OLD_MAX_AGE_GUARD, NEW_MAX_AGE_GUARD, "max-age guard")
    result = replace_once(result, OLD_PROXY_REVALIDATE, b"", "proxy-revalidate max-age guard")
    return result


def composed_source(source: bytes) -> bytes:
    result = replace_once(source, OLD_VARY_MAX_AGE_GUARD, b"", "Vary max-age guard")
    result = replace_once(
        result,
        OLD_NO_CACHE_REVALIDATION,
        NEW_NO_CACHE_REVALIDATION,
        "Vary revalidation guard",
    )
    result = replace_once(
        result,
        OLD_STALE_IF_ERROR,
        NEW_STALE_IF_ERROR,
        "stale-if-error reuse guard",
    )
    result = replace_once(
        result,
        OLD_STALE_WHILE_REVALIDATE,
        NEW_STALE_WHILE_REVALIDATE,
        "stale-while-revalidate reuse guard",
    )
    result = replace_once(
        result,
        OLD_REVALIDATED_POLICY,
        NEW_REVALIDATED_POLICY,
        "stale-if-error request matching guard",
    )
    return result


def patched_source(source: bytes) -> bytes:
    digest = sha256(source)
    if digest == PATCHED_SHA256:
        return source
    if digest == SOURCE_SHA256:
        result = pr58_source(source)
    elif digest == PR58_PATCHED_SHA256:
        result = source
    else:
        raise ValueError(f"unexpected index.js SHA-256: {digest}")

    result = composed_source(result)
    result_hash = sha256(result)
    if result_hash != PATCHED_SHA256:
        raise ValueError(f"patched output SHA-256 mismatch: {result_hash}")
    return result


def package_dirs(root: Path) -> list[Path]:
    if root.is_symlink() or not root.is_dir():
        raise ValueError("root must be a real directory, not a symlink")
    matches = []
    for current, dirs, _ in os.walk(root, followlinks=False):
        current_path = Path(current)
        for name in list(dirs):
            child = current_path / name
            if child.is_symlink():
                if name == PACKAGE_NAME:
                    raise ValueError(f"refusing symlink package directory: {child}")
                dirs.remove(name)
            elif name == PACKAGE_NAME:
                matches.append(child)
                dirs.remove(name)
    return sorted(matches)


def patch_tree(root: Path) -> list[tuple[Path, str]]:
    packages = package_dirs(root)
    if not packages:
        raise ValueError(f"no {PACKAGE_NAME} package found below {root}")

    replacements: list[tuple[Path, bytes, bytes]] = []
    for package_dir in packages:
        manifest_path = package_dir / "package.json"
        source_path = package_dir / "index.js"
        if package_dir.is_symlink() or manifest_path.is_symlink() or source_path.is_symlink():
            raise ValueError(f"refusing symlink package source: {package_dir}")
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as exc:
            raise ValueError(f"cannot read package identity at {manifest_path}: {exc}") from exc
        if manifest.get("name") != PACKAGE_NAME or manifest.get("version") != PACKAGE_VERSION:
            raise ValueError(f"unexpected package identity at {manifest_path}")
        source = source_path.read_bytes()
        output = patched_source(source)
        replacements.append((source_path, source, output))

    # Validate the entire tree before replacing the first file.
    results = []
    for path, old, new in replacements:
        if new != old:
            mode = stat.S_IMODE(path.stat().st_mode)
            fd, tmp_name = tempfile.mkstemp(prefix=".http-cache-semantics-", dir=path.parent)
            tmp_path = Path(tmp_name)
            try:
                with os.fdopen(fd, "wb") as stream:
                    stream.write(new)
                    stream.flush()
                    os.fsync(stream.fileno())
                os.chmod(tmp_path, mode)
                os.replace(tmp_path, path)
                dir_fd = os.open(path.parent, os.O_RDONLY)
                try:
                    os.fsync(dir_fd)
                finally:
                    os.close(dir_fd)
            finally:
                if tmp_path.exists():
                    tmp_path.unlink()
        results.append((path, sha256(new)))
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path, help="installed node_modules root to scan")
    args = parser.parse_args()
    try:
        for path, digest in patch_tree(args.root):
            print(f"{path}: {digest}")
        print("LOCAL MITIGATION APPLIED; package identity remains http-cache-semantics@4.2.0")
        return 0
    except (OSError, ValueError) as exc:
        parser.exit(1, f"cache source mitigation failed: {exc}\n")


if __name__ == "__main__":
    raise SystemExit(main())

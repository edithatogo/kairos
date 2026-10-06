#!/usr/bin/env python3
"""Prepare the hash-locked npm CLI bundle used by the bootstrap tools.

The official npm tarball is integrity-pinned below. The repack moves vulnerable
and security-patched dependencies out of npm's embedded bundle so npm's v3
lock can resolve their full dependency graph and apply exact security overrides. The
generated tarball is local, ignored, and never published.

Run without arguments before the one-time lock refresh. Run with ``--check``
before ``npm ci``: this regenerates the ignored tarball, verifies its integrity
against package-lock.json, and rejects a pre-existing non-reproducible file.
"""

from __future__ import annotations

import argparse
import base64
import gzip
import hashlib
import io
import json
import sys
import tarfile
import urllib.error
import urllib.request
from pathlib import Path


VERSION = "12.1.0"
SOURCE_URL = f"https://registry.npmjs.org/npm/-/npm-{VERSION}.tgz"
SOURCE_INTEGRITY = (
    "sha512-Fyhu62pNx70YCs/5+dEmJQTFVmSKwvo5CA0qvBkGDRpob42MJ6G2RQ2tdxeKM4nYnIZDqkYAxEgqtoejn9QGtQ=="
)
PATCHED_DEPENDENCIES = ("make-fetch-happen", "node-gyp")
POSTCSS_PARSER = "postcss-selector-parser"
POSTCSS_PARSER_VERSION = "7.1.6"
POSTCSS_PARSER_SRI = (
    "sha512-7qASPzhKF2l2KLboRZux8CCTRMdGiV08vWmyKzPz22qZ7ZjQBOeY7rNzNoCLSUiftJ7HUq0GERHmxw/t0dCdMw=="
)
POSTCSS_PARSER_URL = (
    "https://registry.npmjs.org/postcss-selector-parser/-/postcss-selector-parser-7.1.6.tgz"
)
LOCK_SHA256 = "758ec4b68464cafe2e9835ceaee219375fd956b33d59367027ffaec447f8e913"
REMOVED_BUNDLES = (
    *PATCHED_DEPENDENCIES,
    "ip-address",
    "undici",
    "brace-expansion",
    "http-cache-semantics",
    POSTCSS_PARSER,
)
FIXED_DEPENDENCIES = {
    "ip-address": "10.7.1",
    "brace-expansion": "5.0.12",
}
HCS_ALIAS = "@careops/http-cache-semantics-kairos-prototype"
HCS_ALIAS_VERSION = "0.1.0"
HCS_INDEX_SHA256 = "ed6c1faabbe21f7bfef09ce258392cf181678149237a67ce492308a46ca6620c"
HCS_LICENSE_SHA256 = "ab868ad5a2ef5068560d9cd3b2180ec63c140bb4c5cae1ba779d300a0ac74fa3"
MIN_UNDICI = (8, 4, 1)

SCRIPT_DIR = Path(__file__).resolve().parent
PACKAGE_PATH = SCRIPT_DIR / "package.json"
LOCK_PATH = SCRIPT_DIR / "package-lock.json"
OUTPUT_PATH = SCRIPT_DIR / "npm-patched.tgz"


def sri(data: bytes) -> str:
    digest = hashlib.sha512(data).digest()
    return "sha512-" + base64.b64encode(digest).decode("ascii")


def fetch_source() -> bytes:
    request = urllib.request.Request(
        SOURCE_URL,
        headers={"User-Agent": "kairos-bootstrap-npm-preparer/1"},
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            if response.geturl() != SOURCE_URL:
                raise RuntimeError(f"unexpected registry redirect: {response.geturl()}")
            data = response.read()
    except (OSError, urllib.error.URLError) as exc:
        raise RuntimeError(f"could not download pinned npm tarball: {exc}") from exc

    actual = sri(data)
    if actual != SOURCE_INTEGRITY:
        raise RuntimeError(
            f"upstream npm integrity mismatch: expected {SOURCE_INTEGRITY}, got {actual}"
        )
    return data


def is_removed_bundle(path: str) -> bool:
    parts = path.rstrip("/").split("/")
    return any(
        parts[index] == "node_modules" and parts[index + 1] in REMOVED_BUNDLES
        for index in range(len(parts) - 1)
        if index + 1 < len(parts)
    )


def validate_postcss_lock_entries(packages: dict) -> set[str]:
    """Require every lock record for the npm-bundled parser to use the reviewed release."""
    records = {
        path: dependency
        for path, dependency in packages.items()
        if isinstance(path, str)
        and path.rsplit("node_modules/", maxsplit=1)[-1] == POSTCSS_PARSER
        and path.startswith("node_modules/")
    }
    if not records:
        raise RuntimeError("package-lock.json does not resolve the npm transitive parser")
    for path, dependency in records.items():
        parts = path.split("/")
        if any(part in {"", ".", ".."} or "\\" in part for part in parts):
            raise RuntimeError(f"package-lock.json parser path is not canonical: {path}")
        if any(parts[index] != "node_modules" for index, part in enumerate(parts) if part == "node_modules"):
            raise RuntimeError(f"package-lock.json parser path is not canonical: {path}")
        if not isinstance(dependency, dict):
            raise RuntimeError(f"package-lock.json {path} record is invalid")
        if dependency.get("version") != POSTCSS_PARSER_VERSION:
            raise RuntimeError(f"package-lock.json does not pin {path}@{POSTCSS_PARSER_VERSION}")
        if dependency.get("inBundle") is not None and dependency.get("inBundle") is not False:
            raise RuntimeError(f"package-lock.json still marks {path} as bundled")
        if dependency.get("resolved") != POSTCSS_PARSER_URL:
            raise RuntimeError(f"package-lock.json {path} is not the reviewed registry package")
        if dependency.get("integrity") != POSTCSS_PARSER_SRI:
            raise RuntimeError(f"package-lock.json {path} does not match the reviewed package integrity")
    return set(records)


def repack(source_data: bytes) -> bytes:
    source_buffer = io.BytesIO(source_data)
    output_buffer = io.BytesIO()

    with tarfile.open(fileobj=source_buffer, mode="r:gz") as source:
        members = source.getmembers()
        package_manifest_found = False
        source_members = {member.name.rstrip("/"): member for member in members}
        parser_member = source_members.get("package/node_modules/postcss-selector-parser/package.json")
        query_member = source_members.get("package/node_modules/@npmcli/query/package.json")
        if parser_member is None or not parser_member.isfile():
            raise RuntimeError("npm bundle layout changed: transitive postcss-selector-parser package is missing")
        if query_member is None or not query_member.isfile():
            raise RuntimeError("npm bundle layout changed: bundled @npmcli/query package is missing")
        parser_file = source.extractfile(parser_member)
        query_file = source.extractfile(query_member)
        if parser_file is None or query_file is None:
            raise RuntimeError("npm bundle layout changed: transitive parser manifests are unreadable")
        parser_manifest = json.load(parser_file)
        query_manifest = json.load(query_file)
        if (parser_manifest.get("name"), parser_manifest.get("version")) != (POSTCSS_PARSER, "7.1.4"):
            raise RuntimeError("npm bundle layout changed: parser source identity or version drifted")
        if (
            query_manifest.get("name"),
            query_manifest.get("version"),
            query_manifest.get("dependencies", {}).get(POSTCSS_PARSER),
        ) != ("@npmcli/query", "5.0.0", "^7.0.0"):
            raise RuntimeError("npm bundle layout changed: parser is no longer the expected @npmcli/query dependency")
        vulnerable_package_paths: list[str] = []

        with gzip.GzipFile(fileobj=output_buffer, mode="wb", mtime=0) as gzip_output:
            with tarfile.open(
                fileobj=gzip_output,
                mode="w",
                format=tarfile.PAX_FORMAT,
            ) as target:
                for member in members:
                    path = member.name.rstrip("/")
                    if is_removed_bundle(path):
                        continue

                    if path == "package/package.json":
                        package_manifest_found = True
                        manifest_file = source.extractfile(member)
                        if manifest_file is None:
                            raise RuntimeError("npm package manifest is not a regular file")
                        manifest = json.load(manifest_file)
                        if manifest.get("name") != "npm" or manifest.get("version") != VERSION:
                            raise RuntimeError("npm tarball package identity does not match the pin")
                        bundles = manifest.get("bundleDependencies")
                        if not isinstance(bundles, list):
                            raise RuntimeError("npm package no longer has bundleDependencies")
                        for dependency in PATCHED_DEPENDENCIES:
                            if dependency not in bundles:
                                raise RuntimeError(
                                    f"npm bundle layout changed: {dependency} is not a direct bundle"
                                )
                        bundles[:] = [name for name in bundles if name not in REMOVED_BUNDLES]
                        content = (json.dumps(manifest, indent=2, ensure_ascii=False) + "\n").encode()
                    elif member.isfile():
                        source_file = source.extractfile(member)
                        if source_file is None:
                            raise RuntimeError(f"could not read package member {member.name}")
                        content = source_file.read()
                    else:
                        content = None

                    info = tarfile.TarInfo(member.name)
                    info.mode = member.mode
                    info.type = member.type
                    info.linkname = member.linkname
                    info.size = len(content) if content is not None else member.size
                    info.mtime = 0
                    info.uid = 0
                    info.gid = 0
                    info.uname = ""
                    info.gname = ""
                    info.pax_headers = {}
                    target.addfile(
                        info,
                        io.BytesIO(content) if content is not None and member.isfile() else None,
                    )

        if not package_manifest_found:
            raise RuntimeError("npm tarball is missing package/package.json")

        with tarfile.open(fileobj=io.BytesIO(output_buffer.getvalue()), mode="r:gz") as result:
            for member in result.getmembers():
                if is_removed_bundle(member.name):
                    vulnerable_package_paths.append(member.name)
        if vulnerable_package_paths:
            raise RuntimeError(
                "repacked tarball still embeds removed dependencies: "
                + ", ".join(vulnerable_package_paths[:5])
            )

    return output_buffer.getvalue()


def verify_hcs_vendor() -> tuple[str, str]:
    repository_root = SCRIPT_DIR.parent.parent.resolve()
    vendor_root = repository_root / "vendor"
    vendor_dir = vendor_root / "http-cache-semantics-kairos-prototype"
    archive_path = vendor_root / "http-cache-semantics-kairos-prototype-0.1.0.tgz"
    if vendor_root.is_symlink() or vendor_dir.is_symlink() or archive_path.is_symlink():
        raise RuntimeError("local HCS vendor paths must not be symlinks")
    vendor_dir_real = vendor_dir.resolve(strict=True)
    if not vendor_dir_real.is_relative_to(repository_root) or not vendor_dir_real.is_dir():
        raise RuntimeError("local HCS vendor target must be an in-tree real directory")
    manifest_path = vendor_dir_real / "package.json"
    index_path = vendor_dir_real / "index.js"
    license_path = vendor_dir_real / "LICENSE"
    if any(path.is_symlink() or not path.is_file() for path in (manifest_path, index_path, license_path)):
        raise RuntimeError("local HCS vendor manifest, source, and license must be regular files")
    if not archive_path.is_file():
        raise RuntimeError("local HCS vendor archive is missing or not a regular file")

    vendor = json.loads(manifest_path.read_text(encoding="utf-8"))
    if (
        vendor.get("name"), vendor.get("version"), vendor.get("private"), vendor.get("license")
    ) != (HCS_ALIAS, HCS_ALIAS_VERSION, True, "BSD-2-Clause"):
        raise RuntimeError("local HCS vendor identity, privacy, or license metadata drifted")
    if hashlib.sha256(index_path.read_bytes()).hexdigest() != HCS_INDEX_SHA256:
        raise RuntimeError("local HCS vendor index.js digest drifted")
    if hashlib.sha256(license_path.read_bytes()).hexdigest() != HCS_LICENSE_SHA256:
        raise RuntimeError("local HCS vendor BSD license digest drifted")
    archive_data = archive_path.read_bytes()
    archive_integrity = sri(archive_data)
    if hashlib.sha256(archive_data).hexdigest() != "fbd36545bda6d9cd7da805cff6967f96ca2f7f9c59b45e79f97a3e129eec7485":
        raise RuntimeError("local HCS vendor archive digest drifted")
    return "file:../../../../vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz", archive_integrity


def verify_lock(artifact_integrity: str) -> None:
    package = json.loads(PACKAGE_PATH.read_text(encoding="utf-8"))
    lock = json.loads(LOCK_PATH.read_text(encoding="utf-8"))
    npm_lock = lock.get("packages", {}).get("node_modules/npm", {})

    if package.get("dependencies", {}).get("npm") != "file:./npm-patched.tgz":
        raise RuntimeError("package.json must depend on the generated npm-patched.tgz file")
    hcs_override, hcs_integrity = verify_hcs_vendor()
    expected_overrides = {
        **FIXED_DEPENDENCIES,
        POSTCSS_PARSER: POSTCSS_PARSER_VERSION,
        "http-cache-semantics": hcs_override,
    }
    if package.get("overrides") != expected_overrides:
        raise RuntimeError("package.json security overrides do not match the frozen fixed versions and local HCS vendor")
    hcs_lock = lock.get("packages", {}).get("node_modules/http-cache-semantics", {})
    if (
        hcs_lock.get("name"), hcs_lock.get("version"), hcs_lock.get("resolved"),
        hcs_lock.get("integrity"), hcs_lock.get("license"), hcs_lock.get("link", False),
        hcs_lock.get("inBundle", False),
    ) != (
        HCS_ALIAS, HCS_ALIAS_VERSION, "file:../../vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz",
        hcs_integrity, "BSD-2-Clause", False, False,
    ):
        raise RuntimeError("package-lock.json does not bind the exact local private HCS archive")
    if hashlib.sha256(LOCK_PATH.read_bytes()).hexdigest() != LOCK_SHA256:
        raise RuntimeError("package-lock.json differs from the reviewed lock graph")
    if npm_lock.get("version") != VERSION:
        raise RuntimeError("package-lock.json npm version does not match the upstream source pin")
    if npm_lock.get("resolved") != "file:npm-patched.tgz":
        raise RuntimeError("package-lock.json does not resolve npm from npm-patched.tgz")
    if npm_lock.get("integrity") != artifact_integrity:
        raise RuntimeError(
            "generated npm tarball integrity differs from package-lock.json: "
            f"expected {npm_lock.get('integrity')}, got {artifact_integrity}"
        )

    packages = lock.get("packages", {})
    validate_postcss_lock_entries(packages)
    for name, version in FIXED_DEPENDENCIES.items():
        dependency = packages.get(f"node_modules/{name}", {})
        if dependency.get("version") != version or dependency.get("inBundle"):
            raise RuntimeError(f"package-lock.json does not pin {name}@{version}")
        resolved = dependency.get("resolved", "")
        if not resolved.startswith("https://registry.npmjs.org/"):
            raise RuntimeError(f"package-lock.json {name} entry is not registry-resolved")
        if not dependency.get("integrity", "").startswith("sha512-"):
            raise RuntimeError(f"package-lock.json {name} entry has no SHA-512 integrity")

    vulnerable_entries = []
    for path, dependency in packages.items():
        name = path.rsplit("node_modules/", maxsplit=1)[-1]
        version = dependency.get("version", "")
        if name in FIXED_DEPENDENCIES and version != FIXED_DEPENDENCIES[name]:
            vulnerable_entries.append(f"{path}@{version}")
        if name in FIXED_DEPENDENCIES and dependency.get("inBundle"):
            vulnerable_entries.append(f"{path}@{version} (still bundled)")
        if name == POSTCSS_PARSER and version != POSTCSS_PARSER_VERSION:
            vulnerable_entries.append(f"{path}@{version}")
        if name == POSTCSS_PARSER and dependency.get("inBundle"):
            vulnerable_entries.append(f"{path}@{version} (still bundled)")
        if name == "brace-expansion" and dependency.get("inBundle"):
            vulnerable_entries.append(f"{path}@{version} (still bundled)")
        if name == "undici":
            if dependency.get("inBundle"):
                vulnerable_entries.append(f"{path}@{version} (still bundled)")
            try:
                parsed = tuple(int(piece) for piece in version.split(".")[:3])
            except ValueError:
                parsed = ()
            if parsed < MIN_UNDICI:
                vulnerable_entries.append(f"{path}@{version} (below npm 12 node-gyp range)")
        if name in PATCHED_DEPENDENCIES and dependency.get("inBundle"):
            vulnerable_entries.append(f"{path}@{version} (still bundled)")
    if vulnerable_entries:
        raise RuntimeError(
            "package-lock.json contains stale bundled or vulnerable packages: "
            + ", ".join(vulnerable_entries)
        )

    for name in PATCHED_DEPENDENCIES:
        dependency = packages.get(f"node_modules/{name}", {})
        if dependency.get("inBundle") or not dependency.get("resolved", "").startswith(
            "https://registry.npmjs.org/"
        ):
            raise RuntimeError(f"package-lock.json {name} is not resolved as a registry package")
    npm_dependencies = npm_lock.get("dependencies", {})
    if not set(PATCHED_DEPENDENCIES).issubset(npm_dependencies):
        raise RuntimeError("npm lock entry does not declare the expected unbundled dependencies")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--check",
        action="store_true",
        help="verify source, generated bytes, embedded contents, and committed lock integrity",
    )
    mode.add_argument("--write", action="store_true", help="write the deterministic local repack")
    args = parser.parse_args()

    try:
        patched = repack(fetch_source())
        patched_integrity = sri(patched)

        if args.check:
            verify_lock(patched_integrity)
            if OUTPUT_PATH.exists() and OUTPUT_PATH.read_bytes() != patched:
                raise RuntimeError("existing npm-patched.tgz is not byte-identical to regenerated output")

        OUTPUT_PATH.write_bytes(patched)
        print(f"Prepared npm@{VERSION}: {OUTPUT_PATH}")
        print(f"Source integrity: {SOURCE_INTEGRITY}")
        print(f"Generated integrity: {patched_integrity}")
        if args.check:
            print("Lock integrity, fixed dependency versions, and embedded-package exclusion: PASS")
        return 0
    except (OSError, ValueError, RuntimeError, tarfile.TarError) as exc:
        print(f"npm CLI preparation failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

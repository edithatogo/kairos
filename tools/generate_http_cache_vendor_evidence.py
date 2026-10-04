#!/usr/bin/env python3
"""Generate or verify the private HTTP-cache vendor evidence artifacts."""
from __future__ import annotations

import argparse
import base64
import gzip
import hashlib
import importlib.metadata
import io
import json
import re
import stat
import tarfile
import uuid
import zipfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
PACKAGE_DIR = ROOT / "vendor/http-cache-semantics-kairos-prototype"
PACKAGE_TARBALL = ROOT / "vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz"
INPUT_ZIP = PACKAGE_DIR / "vendor-evidence-inputs.zip"
SBOM_PATH = PACKAGE_DIR / "sbom.cdx.json"
PROVENANCE_PATH = PACKAGE_DIR / "provenance.intoto.json"

PACKAGE_NAME = "@careops/http-cache-semantics-kairos-prototype"
PACKAGE_VERSION = "0.1.0"
PACKAGE_PURL = "pkg:npm/%40careops/http-cache-semantics-kairos-prototype@0.1.0"
UPSTREAM_NAME = "http-cache-semantics"
UPSTREAM_VERSION = "4.2.0"
UPSTREAM_PURL = "pkg:npm/http-cache-semantics@4.2.0"
PATCHED_ARCHIVE_SHA256 = "fbd36545bda6d9cd7da805cff6967f96ca2f7f9c59b45e79f97a3e129eec7485"
EVIDENCE_ZIP_SHA256 = "1efb1d1bcf379111854e49e50b3371027bf422a529eadcd69efec4ccdcf3fab8"
QUALIFICATION_SHA256 = "41f2137e9dc6c12023e13eb55be32ef8588ceabd1a511423332f0d9ddf6372f1"
GENERATOR_VERSION = "1.0.0"
VALIDATOR_DISTRIBUTIONS = {"jsonschema": "4.26.0", "attrs": "26.1.0", "jsonschema-specifications": "2025.9.1", "referencing": "0.37.0", "rpds-py": "2026.5.1"}

PACKAGE_FILES = {
    "package.json": "0159915b5e6dac00760bf8ce50daebfcf441cbdb66ef6aa8f05017248b4ccb42",
    "index.js": "ed6c1faabbe21f7bfef09ce258392cf181678149237a67ce492308a46ca6620c",
    "LICENSE": "ab868ad5a2ef5068560d9cd3b2180ec63c140bb4c5cae1ba779d300a0ac74fa3",
    "README.md": "0039d96fcc60d2065d2cdae27cc159c74e6fc7380a557d138f0c54695a06c01b",
}
EVIDENCE_MEMBERS = {
    "NOTICE.txt": "bd0e6bf9cb98be89701849bec2058a3642f5da35d14c66db33e1532f292eeeb1",
    "advisories/GHSA-ch52-4w7c-c8xp.json": "1871b8186dd10aa82aa211741c98a71e0209f328efe9a23aee1494f765c3a6e4",
    "licenses/CycloneDX-Apache-2.0.txt": "6c29f22a4a7385285c6f579ec9f33c5e989f00739d6b257243a0b082ec9447ae",
    "licenses/SLSA-Community-Specification-1.0.md": "f40b1d00369c21b963293189b4617b5de439a11858541c07c620984634fb63b7",
    "patches/PR1-101a9e9a.patch": "5c4868499d8c5eb55985b2515cc309d9ba3fed86a6f2070f4bcdc859e41f7c64",
    "patches/PR58-14a8c2ad.patch": "4f44c4381cc2ea7406e559152e031c397068547259d270a00ca3199a8cac25fc",
    "registry/http-cache-semantics-4.2.0.json": "1813cd94a97f5329db475853f0bf0b532bffb9905592eea8a080f593503ad8a3",
    "registry/http-cache-semantics-4.2.0.tgz": "f57454db8ab2d06a4baabbfb7c70ad4181a00b7ee0efebcdffa9ba313bd80eb7",
    "schemas/cyclonedx/1.7/bom-1.7.schema.json": "df472ef4aaf593904c479293723a1a5c191d6672715c93b3c0b5c318f3914221",
    "schemas/cyclonedx/1.7/cryptography-defs.schema.json": "018ea7f78b5208ec647cfd10f669cc9c26aba6aceb79c4da7f9c0ef4c99b60de",
    "schemas/cyclonedx/1.7/jsf-0.82.schema.json": "8bae002c25e723db7ee1f26afde680ae1a2b1a8f6b4b4b0fd65dc3becb090aae",
    "schemas/cyclonedx/1.7/spdx.schema.json": "54a6288292bc6c90b0d3952f5f939f17436fa76704ffe68a46e5b78539c7cc1b",
    "schemas/slsa/1.2/provenance.cue": "0ddd00372622d1c04f71d1fb2666579af089eb733c9c5d838d158489b2ea6145",
}
NOTICE_TEXT = (
    "This private CareOps prototype contains a modified fork derived from http-cache-semantics 4.2.0. "
    "The upstream BSD-2-Clause license and source identity are retained; local security changes are "
    "documented by the included patches. This artifact is not an upstream release and does not assert "
    "public advisory closure.\n"
)

UPSTREAM_TARBALL_URL = "https://registry.npmjs.org/http-cache-semantics/-/http-cache-semantics-4.2.0.tgz"
UPSTREAM_METADATA_URL = "https://registry.npmjs.org/http-cache-semantics/4.2.0"
GHSA_URL = "https://api.github.com/advisories/GHSA-ch52-4w7c-c8xp"
PATCHES = [
    ("101a9e9a5b9aba5750a74b8f659c5646e90a962f", "https://github.com/hellonewday/http-cache-semantics/commit/101a9e9a5b9aba5750a74b8f659c5646e90a962f.patch", EVIDENCE_MEMBERS["patches/PR1-101a9e9a.patch"]),
    ("14a8c2ad51740dc39bf3e8f1a11c845a5003f217", "https://github.com/kornelski/http-cache-semantics/commit/14a8c2ad51740dc39bf3e8f1a11c845a5003f217.patch", EVIDENCE_MEMBERS["patches/PR58-14a8c2ad.patch"]),
]


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def json_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def read_package_payload(root: Path = ROOT) -> dict[str, bytes]:
    payload: dict[str, bytes] = {}
    for name, expected in PACKAGE_FILES.items():
        data = (root / "vendor/http-cache-semantics-kairos-prototype" / name).read_bytes()
        if sha256(data) != expected:
            raise ValueError(f"package input hash mismatch: {name}")
        payload[name] = data
    return payload


def build_package_archive(payload: dict[str, bytes]) -> bytes:
    """Rebuild npm archive bytes using only the four accepted package payload files."""
    if set(payload) != set(PACKAGE_FILES):
        raise ValueError("package payload must contain exactly the four allowlisted files")
    out = io.BytesIO()
    with gzip.GzipFile(fileobj=out, mode="wb", filename="", mtime=0, compresslevel=9) as gz:
        with tarfile.open(fileobj=gz, mode="w", format=tarfile.USTAR_FORMAT) as archive:
            directory = tarfile.TarInfo("package")
            directory.type = tarfile.DIRTYPE
            directory.mode = 0o755
            directory.mtime = directory.uid = directory.gid = 0
            directory.uname = directory.gname = ""
            archive.addfile(directory)
            for arcname, source in [
                ("package/package.json", "package.json"),
                ("package/index.js", "index.js"),
                ("package/LICENSE", "LICENSE"),
                ("package/README.md", "README.md"),
            ]:
                data = payload[source]
                entry = tarfile.TarInfo(arcname)
                entry.size = len(data)
                entry.mode = 0o644
                entry.mtime = entry.uid = entry.gid = 0
                entry.uname = entry.gname = ""
                archive.addfile(entry, io.BytesIO(data))
    result = out.getvalue()
    if sha256(result) != PATCHED_ARCHIVE_SHA256:
        raise ValueError("four-file deterministic npm archive does not match accepted archive hash")
    return result


def read_evidence_members(data: bytes) -> dict[str, bytes]:
    if sha256(data) != EVIDENCE_ZIP_SHA256:
        raise ValueError("evidence ZIP digest mismatch")
    result: dict[str, bytes] = {}
    with zipfile.ZipFile(io.BytesIO(data), "r") as archive:
        infos = archive.infolist()
        names = [info.filename for info in infos]
        if names != sorted(EVIDENCE_MEMBERS) or len(names) != len(set(names)):
            raise ValueError("evidence ZIP has missing, extra, duplicate, or unordered members")
        for info in infos:
            mode = (info.external_attr >> 16) & 0xFFFF
            if info.is_dir() or stat.S_ISLNK(mode) or stat.S_IFMT(mode) != stat.S_IFREG:
                raise ValueError(f"evidence ZIP member is not a regular file: {info.filename}")
            if stat.S_IMODE(mode) != 0o644 or info.date_time != (1980, 1, 1, 0, 0, 0):
                raise ValueError(f"evidence ZIP member has unexpected mode or timestamp: {info.filename}")
            if info.compress_type != zipfile.ZIP_STORED or info.extra or info.comment:
                raise ValueError(f"evidence ZIP member has unexpected encoding metadata: {info.filename}")
            member = archive.read(info)
            if sha256(member) != EVIDENCE_MEMBERS[info.filename]:
                raise ValueError(f"evidence ZIP member digest mismatch: {info.filename}")
            result[info.filename] = member
    return result


def build_evidence_zip(members: dict[str, bytes]) -> bytes:
    if set(members) != set(EVIDENCE_MEMBERS):
        raise ValueError("evidence inputs must contain exactly the 13 allowlisted members")
    for name, expected in EVIDENCE_MEMBERS.items():
        if sha256(members[name]) != expected:
            raise ValueError(f"evidence input hash mismatch: {name}")
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_STORED, strict_timestamps=True) as archive:
        for name in sorted(members):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_STORED
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | 0o644) << 16
            info.extra = b""
            info.comment = b""
            archive.writestr(info, members[name])
    result = out.getvalue()
    if sha256(result) != EVIDENCE_ZIP_SHA256:
        raise ValueError("deterministic evidence ZIP hash mismatch")
    if "sha512-" + base64.b64encode(hashlib.sha512(result).digest()).decode("ascii") != "sha512-ChC/C+oe4IA8hrDnsiqd7zg3D08Mj/eQmKRuuSXI2wvJC8JvMmFfsQ3kZmmdfPAuja5IMIKOzzeBdNjmSO654Q==":
        raise ValueError("evidence ZIP SRI mismatch")
    return result


def resource(uri: str, digest: str, name: str | None = None) -> dict[str, Any]:
    value: dict[str, Any] = {"uri": uri, "digest": {"sha256": digest}}
    if name:
        value["name"] = name
    return value


def build_sbom(archive_sha: str = PATCHED_ARCHIVE_SHA256) -> dict[str, Any]:
    serial = str(uuid.uuid5(uuid.NAMESPACE_URL, f"{PACKAGE_PURL}:{archive_sha}"))
    upstream = {
        "type": "library",
        "bom-ref": UPSTREAM_PURL,
        "name": UPSTREAM_NAME,
        "version": UPSTREAM_VERSION,
        "purl": UPSTREAM_PURL,
        "licenses": [{"license": {"id": "BSD-2-Clause"}}],
        "externalReferences": [{"type": "distribution", "url": UPSTREAM_TARBALL_URL}],
    }
    component = {
        "type": "library",
        "bom-ref": PACKAGE_PURL,
        "name": PACKAGE_NAME,
        "version": PACKAGE_VERSION,
        "purl": PACKAGE_PURL,
        "scope": "required",
        "description": "Private CareOps prototype fork; not an upstream release.",
        "licenses": [{"license": {"id": "BSD-2-Clause"}}],
        "hashes": [{"alg": "SHA-256", "content": archive_sha}],
        "externalReferences": [
            {"type": "distribution", "url": "file:vendor"},
            {"type": "issue-tracker", "url": GHSA_URL, "comment": "Upstream advisory remains open in the captured source snapshot; no public closure is claimed."},
        ],
        "pedigree": {
            "ancestors": [upstream],
            "commits": [
                {"uid": PATCHES[0][0], "url": PATCHES[0][1], "message": "Upstream pull request patch applied to private prototype"},
                {"uid": PATCHES[1][0], "url": PATCHES[1][1], "message": "Upstream pull request patch applied to private prototype"},
            ],
            "patches": [
                {"type": "unofficial", "diff": {"url": PATCHES[0][1]}},
                {"type": "unofficial", "diff": {"url": PATCHES[1][1]}},
            ],
            "notes": "Derived from http-cache-semantics 4.2.0. This private patched fork retains the upstream identity and does not imply an upstream release or public advisory closure.",
        },
        "properties": [
            {"name": "careops:upstream-advisory", "value": "GHSA-ch52-4w7c-c8xp"},
            {"name": "careops:upstream-cve", "value": "CVE-2026-93748"},
            {"name": "careops:upstream-affected-range", "value": "<= 4.2.0"},
            {"name": "careops:public-advisory-closure", "value": "not claimed"},
        ],
    }
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.7",
        "serialNumber": "urn:uuid:" + serial,
        "version": 1,
        "metadata": {
            "tools": {"components": [{"type": "application", "name": "careops-vendor-evidence-generator", "version": GENERATOR_VERSION}]},
            "component": component,
        },
        "components": [],
    }


def _resolved_dependencies(members: dict[str, bytes], payload: dict[str, bytes], archive_sha: str, evidence_sha: str) -> list[dict[str, Any]]:
    dependencies = [
        resource(UPSTREAM_TARBALL_URL, EVIDENCE_MEMBERS["registry/http-cache-semantics-4.2.0.tgz"], "http-cache-semantics-4.2.0.tgz"),
        resource(UPSTREAM_METADATA_URL, EVIDENCE_MEMBERS["registry/http-cache-semantics-4.2.0.json"], "http-cache-semantics@4.2.0 registry metadata"),
        resource(GHSA_URL, EVIDENCE_MEMBERS["advisories/GHSA-ch52-4w7c-c8xp.json"], "GHSA-ch52-4w7c-c8xp / CVE-2026-93748"),
        resource("file:vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz", archive_sha, "private patched npm archive"),
        resource("file:vendor/http-cache-semantics-kairos-prototype/vendor-evidence-inputs.zip", evidence_sha, "frozen evidence source archive"),
    ]
    for filename in PACKAGE_FILES:
        dependencies.append(resource(f"file:vendor/http-cache-semantics-kairos-prototype/{filename}", sha256(payload[filename]), filename))
    for commit, url, digest in PATCHES:
        dependencies.append(resource(url, digest, f"patch {commit}"))
    source_urls = {
        "licenses/CycloneDX-Apache-2.0.txt": "https://raw.githubusercontent.com/CycloneDX/specification/4b3f59453366e27c8073fd24e98bf21ef8892c8e/LICENSE",
        "licenses/SLSA-Community-Specification-1.0.md": "https://raw.githubusercontent.com/slsa-framework/slsa/19e4e2f005f871270c4f555fc47afecfb37f3efe/LICENSE.md",
        "schemas/cyclonedx/1.7/bom-1.7.schema.json": "https://raw.githubusercontent.com/CycloneDX/specification/4b3f59453366e27c8073fd24e98bf21ef8892c8e/schema/bom-1.7.schema.json",
        "schemas/cyclonedx/1.7/cryptography-defs.schema.json": "https://raw.githubusercontent.com/CycloneDX/specification/4b3f59453366e27c8073fd24e98bf21ef8892c8e/schema/cryptography-defs.schema.json",
        "schemas/cyclonedx/1.7/jsf-0.82.schema.json": "https://raw.githubusercontent.com/CycloneDX/specification/4b3f59453366e27c8073fd24e98bf21ef8892c8e/schema/jsf-0.82.schema.json",
        "schemas/cyclonedx/1.7/spdx.schema.json": "https://raw.githubusercontent.com/CycloneDX/specification/4b3f59453366e27c8073fd24e98bf21ef8892c8e/schema/spdx.schema.json",
        "schemas/slsa/1.2/provenance.cue": "https://raw.githubusercontent.com/slsa-framework/slsa/19e4e2f005f871270c4f555fc47afecfb37f3efe/docs/spec/v1.2/schema/provenance.cue",
    }
    for name, digest in EVIDENCE_MEMBERS.items():
        if name in source_urls:
            dependencies.append(resource(source_urls[name], digest, name))
    # Keep the actual pinned source URIs for schema/license inputs in the bundle.
    return sorted(dependencies, key=lambda item: (item["uri"], item["digest"]["sha256"]))


def build_provenance(payload: dict[str, bytes], members: dict[str, bytes], archive: bytes, evidence_zip: bytes, sbom: bytes, script_sha256: str) -> dict[str, Any]:
    archive_sha = sha256(archive)
    evidence_sha = sha256(evidence_zip)
    deps = _resolved_dependencies(members, payload, archive_sha, evidence_sha)
    statement = {
        "_type": "https://in-toto.io/Statement/v1",
        "subject": [{"name": PACKAGE_DIR.name + "-0.1.0.tgz", "digest": {"sha256": archive_sha}}],
        "predicateType": "https://slsa.dev/provenance/v1",
        "predicate": {
            "buildDefinition": {
                "buildType": "urn:careops:build-type:private-npm-vendor-evidence:v1",
                "externalParameters": {
                    "historicalQualificationReceiptSha256": QUALIFICATION_SHA256,
                    "qualificationStatus": "historical receipt excludes local Connection parser hardening; fresh CI required",
                    "privatePackageName": PACKAGE_NAME,
                    "privatePackageVersion": PACKAGE_VERSION,
                    "upstreamPackage": UPSTREAM_PURL,
                    "publicAdvisoryClosure": "not claimed",
                    "buildTypeStatus": "local-undocumented",
                    "buildTypeMapping": "No external standard or consumer mapping is claimed.",
                },
                "internalParameters": {
                    "archiveFormat": "USTAR inside gzip with zero timestamps and normalized ownership/mode",
                    "evidenceZipFormat": "ZIP_STORED, lexicographic paths, 1980 timestamp, regular 0644 entries",
                    "evidenceZipSha256": evidence_sha,
                    "generatorSha256": script_sha256,
                    "sbomSha256": sha256(sbom),
                },
                "resolvedDependencies": deps,
            },
            "runDetails": {
                "builder": {
                    "id": "urn:careops:local-untrusted-builder:vendor-evidence",
                    "builderDependencies": [],
                    "version": {"generator": GENERATOR_VERSION, "profile": "local-unsigned-evidence"},
                },
                "metadata": {
                    "invocationId": f"vendor-evidence-{archive_sha[:16]}-{evidence_sha[:16]}",
                },
                "byproducts": [
                    resource("file:vendor-evidence-inputs.zip", evidence_sha, "evidence input archive"),
                    resource("file:sbom.cdx.json", sha256(sbom), "CycloneDX BOM"),
                ],
            },
        },
    }
    return statement


def derived_slsa_cue(official: bytes) -> bytes:
    """Adapt the pinned informative CUE summary for JSON and normative fields.

    Keep field types while making ResourceDescriptor fields and RFC3339 metadata
    times optional, matching the normative one-of and optional-time semantics.
    The normative profile validates one-of values and RFC3339 when times exist.
    """
    text = official.decode("utf-8")
    replacements = [
        ('"version": { ...string }', '"version": { [string]: string }'),
        ('"externalParameters": object', '"externalParameters": {...}'),
        ('"internalParameters": object', '"internalParameters": {...}'),
        ('"uri": string,', '"uri"?: string,'),
        ('"digest": {', '"digest"?: {'),
        ('"sha256": string,', '"sha256"?: string,'),
        ('"sha512": string,', '"sha512"?: string,'),
        ('"gitCommit": string,', '"gitCommit"?: string,'),
        ('"name": string,', '"name"?: string,'),
        ('"downloadLocation": string,', '"downloadLocation"?: string,'),
        ('"mediaType": string,', '"mediaType"?: string,'),
        ('"content": bytes', '"content"?: string'),
        ('"annotations": {', '"annotations"?: {'),
        ('"startedOn": #Timestamp,', '"startedOn"?: #Timestamp,'),
        ('"finishedOn": #Timestamp,', '"finishedOn"?: #Timestamp,'),
    ]
    for before, after in replacements:
        if text.count(before) != 1:
            raise ValueError("pinned informative SLSA CUE source changed at adapter target: " + before)
        text = text.replace(before, after, 1)
    if sha256(text.encode("utf-8")) != "4d4a43d17e5f80c238f4a8a6f22bc459b611eaf646a45235fa83d3b159a4aff2":
        raise ValueError("derived SLSA CUE schema digest mismatch")
    return text.encode("utf-8")


def _validate_resource_descriptors(value: Any, label: str) -> None:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    for index, item in enumerate(value):
        where = f"{label}[{index}]"
        if not isinstance(item, dict):
            raise ValueError(f"{where} ResourceDescriptor must be an object")
        for field in ("uri", "name", "downloadLocation", "mediaType"):
            if field in item and (not isinstance(item[field], str) or not item[field]):
                raise ValueError(f"{where}.{field} must be a nonempty string when present")
        has_uri = "uri" in item
        has_content = "content" in item
        has_digest = "digest" in item
        if not (has_uri or has_content or has_digest):
            raise ValueError(f"{where} requires uri, digest, or content")
        if has_digest:
            digest = item["digest"]
            if not isinstance(digest, dict) or not digest or any(not isinstance(v, str) or not v for v in digest.values()):
                raise ValueError(f"{where}.digest must be a nonempty string map")
        if has_content:
            encoded = item["content"]
            if not isinstance(encoded, str):
                raise ValueError(f"{where}.content must be a base64 string when present")
            try:
                decoded = base64.b64decode(encoded, validate=True)
            except Exception as exc:
                raise ValueError(f"{where}.content is not strict base64") from exc
            if base64.b64encode(decoded).decode("ascii") != encoded:
                raise ValueError(f"{where}.content base64 is not canonical")
        if "annotations" in item and not isinstance(item["annotations"], dict):
            raise ValueError(f"{where}.annotations must be an object when present")


def _validate_rfc3339(value: Any, label: str) -> None:
    if not isinstance(value, str) or not re.fullmatch(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})", value
    ):
        raise ValueError(f"{label} must be an RFC3339 timestamp with an explicit offset")
    from datetime import datetime
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError(f"{label} is not a valid RFC3339 calendar timestamp") from exc
    if parsed.tzinfo is None:
        raise ValueError(f"{label} must include a timezone offset")


def validate_normative_profile(statement: dict[str, Any]) -> None:
    if not isinstance(statement, dict) or statement.get("_type") != "https://in-toto.io/Statement/v1":
        raise ValueError("in-toto Statement v1 _type missing or invalid")
    subjects = statement.get("subject")
    if not isinstance(subjects, list) or not subjects:
        raise ValueError("statement subject must be a nonempty list")
    for subject in subjects:
        if not isinstance(subject, dict) or not isinstance(subject.get("name"), str) or not subject["name"]:
            raise ValueError("subject requires name")
        digest = subject.get("digest")
        if not isinstance(digest, dict) or not digest or any(not isinstance(v, str) or not v for v in digest.values()):
            raise ValueError("subject requires a nonempty digest map")
    if statement.get("predicateType") != "https://slsa.dev/provenance/v1":
        raise ValueError("SLSA provenance v1 predicateType missing or invalid")
    predicate = statement.get("predicate")
    if not isinstance(predicate, dict):
        raise ValueError("predicate must be an object")
    build = predicate.get("buildDefinition")
    if not isinstance(build, dict) or not isinstance(build.get("buildType"), str) or not build["buildType"]:
        raise ValueError("buildDefinition and buildType are required")
    if not isinstance(build.get("externalParameters"), dict):
        raise ValueError("buildDefinition.externalParameters must be an object")
    if "resolvedDependencies" in build:
        _validate_resource_descriptors(build["resolvedDependencies"], "buildDefinition.resolvedDependencies")
    run = predicate.get("runDetails")
    if not isinstance(run, dict):
        raise ValueError("runDetails is required")
    builder = run.get("builder")
    if not isinstance(builder, dict) or not isinstance(builder.get("id"), str) or not builder["id"]:
        raise ValueError("runDetails.builder.id is required")
    if "builderDependencies" in builder:
        _validate_resource_descriptors(builder["builderDependencies"], "runDetails.builder.builderDependencies")
    if "byproducts" in run:
        _validate_resource_descriptors(run["byproducts"], "runDetails.byproducts")
    metadata = run.get("metadata")
    if metadata is not None:
        if not isinstance(metadata, dict):
            raise ValueError("runDetails.metadata must be an object when present")
        for field in ("startedOn", "finishedOn"):
            if field in metadata:
                _validate_rfc3339(metadata[field], f"runDetails.metadata.{field}")


def cyclonedx_validator(members: dict[str, bytes]):
    for distribution, expected in VALIDATOR_DISTRIBUTIONS.items():
        try:
            actual = importlib.metadata.version(distribution)
        except importlib.metadata.PackageNotFoundError as exc:
            raise RuntimeError(f"full CycloneDX validation requires pinned {distribution} {expected}") from exc
        if actual != expected:
            raise RuntimeError(f"validator distribution drift: {distribution} {actual} != {expected}")
    try:
        from jsonschema import Draft7Validator
        from referencing import Registry, Resource
    except ImportError as exc:
        raise RuntimeError("full CycloneDX validation requires pinned jsonschema 4.26.0 and runtime dependencies") from exc
    schemas = {}
    for name in sorted(EVIDENCE_MEMBERS):
        if not name.endswith(".schema.json"):
            continue
        schema = json.loads(members[name])
        schemas[schema["$id"]] = schema
    root_schema = json.loads(members["schemas/cyclonedx/1.7/bom-1.7.schema.json"])
    registry = Registry()
    for uri, schema in schemas.items():
        registry = registry.with_resource(uri, Resource.from_contents(schema))
    Draft7Validator.check_schema(root_schema)
    return Draft7Validator(root_schema, registry=registry)


def build_outputs(root: Path = ROOT) -> dict[str, bytes]:
    payload = read_package_payload(root)
    archive = build_package_archive(payload)
    existing_archive = (root / "vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz").read_bytes()
    if existing_archive != archive:
        raise ValueError("committed patched archive differs from four-file deterministic recreation")
    input_zip = (root / "vendor/http-cache-semantics-kairos-prototype/vendor-evidence-inputs.zip").read_bytes()
    members = read_evidence_members(input_zip)
    rebuilt_zip = build_evidence_zip(members)
    sbom_obj = build_sbom(sha256(archive))
    validator = cyclonedx_validator(members)
    errors = list(validator.iter_errors(sbom_obj))
    if errors:
        raise ValueError("generated CycloneDX BOM fails full schema: " + "; ".join(e.message for e in errors[:5]))
    sbom = json_bytes(sbom_obj)
    script_hash = sha256(Path(__file__).read_bytes())
    provenance_obj = build_provenance(payload, members, archive, rebuilt_zip, sbom, script_hash)
    validate_normative_profile(provenance_obj)
    provenance = json_bytes(provenance_obj)
    return {"zip": rebuilt_zip, "sbom": sbom, "provenance": provenance}


def write_outputs(outputs: dict[str, bytes], root: Path = ROOT) -> None:
    for key, path in [("zip", INPUT_ZIP), ("sbom", SBOM_PATH), ("provenance", PROVENANCE_PATH)]:
        destination = root / path.relative_to(ROOT)
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(outputs[key])


def check_outputs(outputs: dict[str, bytes], root: Path = ROOT) -> None:
    for key, path in [("zip", INPUT_ZIP), ("sbom", SBOM_PATH), ("provenance", PROVENANCE_PATH)]:
        destination = root / path.relative_to(ROOT)
        if not destination.is_file() or destination.read_bytes() != outputs[key]:
            raise ValueError(f"generated output differs from frozen file: {destination.relative_to(root)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--write", action="store_true", help="write deterministic generated output files")
    action.add_argument("--check", action="store_true", help="verify exact generated bytes without writing")
    args = parser.parse_args()
    try:
        outputs = build_outputs()
        if args.write:
            write_outputs(outputs)
        else:
            check_outputs(outputs)
    except Exception as exc:
        parser.exit(1, f"vendor evidence validation failed: {exc}\n")
    print("vendor evidence outputs verified" if args.check else "vendor evidence outputs written")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

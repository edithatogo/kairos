#!/usr/bin/env python3
"""Independent member, manifest, metadata, and byte reproducibility audit."""
from __future__ import annotations

import gzip
import hashlib
import io
import json
import tarfile
from pathlib import PurePosixPath
import sys

EXPECTED_SHA256 = "88732ce4024f3f4040a1c625ade58d98cceb810721ef06ce68c408ea2bae8218"
EXPECTED_HEAD = "ddf93fc0c4348654a127494b975a22dbe2d5ea0b"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def safe_name(name: str) -> bool:
    path = PurePosixPath(name)
    return (
        bool(name)
        and not path.is_absolute()
        and "\\" not in name
        and "\x00" not in name
        and not any(part in ("", ".", "..") for part in name.split("/"))
        and str(path) == name
    )


def main() -> int:
    archive_path = sys.argv[1]
    raw = open(archive_path, "rb").read()
    archive_hash = sha(raw)
    assert archive_hash == EXPECTED_SHA256, f"archive SHA mismatch: {archive_hash}"
    assert raw[:3] == b"\x1f\x8b\x08", "not gzip"
    assert raw[3] == 0 and raw[4:8] == b"\0\0\0\0", "gzip flags/mtime not deterministic"

    tar_bytes = gzip.decompress(raw)
    with tarfile.open(fileobj=io.BytesIO(tar_bytes), mode="r:") as archive:
        members = archive.getmembers()
        assert len(members) == 585, f"expected 585 members, got {len(members)}"
        names = [member.name for member in members]
        assert names[0] == "MANIFEST.json", "manifest must be first"
        assert names[1:] == sorted(names[1:]), "archive member order is not sorted"
        assert len(set(names)) == len(names), "duplicate tar member names"
        assert all(safe_name(name) for name in names), "unsafe archive member path"

        content: dict[str, bytes] = {}
        metadata = []
        for member in members:
            assert member.isfile() and member.type == tarfile.REGTYPE, f"non-regular member: {member.name}"
            assert not member.pax_headers, f"unexpected PAX headers: {member.name}"
            assert (member.mode, member.uid, member.gid, member.uname, member.gname, member.mtime) == (
                0o644, 0, 0, "", "", 0
            ), f"nondeterministic tar metadata: {member.name}"
            stream = archive.extractfile(member)
            assert stream is not None
            value = stream.read()
            assert len(value) == member.size, f"short member read: {member.name}"
            content[member.name] = value
            metadata.append({
                "name": member.name,
                "bytes": member.size,
                "mode": member.mode,
                "uid": member.uid,
                "gid": member.gid,
                "mtime": member.mtime,
                "type": "regular",
                "sha256": sha(value),
            })

    manifest_bytes = content["MANIFEST.json"]
    manifest = json.loads(manifest_bytes)
    assert set(manifest) == {"excluded", "members", "source_commit"}, "unexpected manifest keys"
    assert manifest["source_commit"] == EXPECTED_HEAD, "manifest source commit mismatch"
    expected_names = sorted(content.keys() - {"MANIFEST.json"})
    declared = manifest["members"]
    assert list(declared) == expected_names, "manifest member names/order mismatch"
    for name in expected_names:
        entry = declared[name]
        value = content[name]
        assert set(entry) == {"bytes", "sha256"}, f"unexpected manifest entry fields: {name}"
        assert entry["bytes"] == len(value), f"manifest size mismatch: {name}"
        assert entry["sha256"] == sha(value), f"manifest SHA mismatch: {name}"

    tar_out = io.BytesIO()
    with tarfile.open(fileobj=tar_out, mode="w", format=tarfile.USTAR_FORMAT) as rebuilt:
        for name in names:
            value = content[name]
            info = tarfile.TarInfo(name)
            info.size = len(value)
            info.mode = 0o644
            info.uid = info.gid = info.mtime = 0
            info.uname = info.gname = ""
            info.type = tarfile.REGTYPE
            info.pax_headers = {}
            rebuilt.addfile(info, io.BytesIO(value))
    gzip_out = io.BytesIO()
    with gzip.GzipFile(fileobj=gzip_out, filename="", mode="wb", compresslevel=9, mtime=0) as compressed:
        compressed.write(tar_out.getvalue())
    reproduced = gzip_out.getvalue()
    assert reproduced == raw, f"deterministic archive bytes differ (rebuilt SHA {sha(reproduced)})"
    assert len(metadata) == 585
    print(json.dumps({
        "archive": archive_path,
        "archive_bytes": len(raw),
        "archive_sha256": archive_hash,
        "head": EXPECTED_HEAD,
        "member_count_including_manifest": len(members),
        "manifest_member_count": len(declared),
        "members_sorted_unique_safe_regular": True,
        "manifest_sizes_and_sha256_exact": True,
        "metadata": {"gzip_mtime": 0, "gzip_flags": 0, "tar_mode": "0644", "uid": 0, "gid": 0, "mtime": 0, "pax": False},
        "reproduced_bytes_equal": True,
        "reproduced_sha256": sha(reproduced),
    }, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

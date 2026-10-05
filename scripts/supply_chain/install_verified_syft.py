#!/usr/bin/env python3
"""Install Syft only after authenticating Anchore's signed release checksums."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import math
import os
import platform
import re
import signal
import ssl
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path, PurePosixPath
from typing import Any, Callable

VERSION = "1.54.0"
RELEASE_TAG = "v1.54.0"
RELEASE_COMMIT = "cc326e45a6213360266dda4b30cc68095946d676"
REPOSITORY = "anchore/syft"
WORKFLOW_REF = "refs/heads/main"
CERT_IDENTITY = "https://github.com/anchore/syft/.github/workflows/release.yaml@refs/heads/main"
OIDC_ISSUER = "https://token.actions.githubusercontent.com"
CHECKSUM_URL = "https://github.com/anchore/syft/releases/download/v1.54.0/syft_1.54.0_checksums.txt"
BUNDLE_URL = "https://github.com/anchore/syft/releases/download/v1.54.0/syft_1.54.0_checksums.txt.sigstore.json"
CHECKSUM_SHA256 = "e423344e663d7d14db62e51ddd31e4a5012818ed69e78358391dc487483f839c"
BUNDLE_SHA256 = "6a0dbf94cb89e2fb157f022bed752b3ceb5827cb557a4a4cfb6af67f98b99811"
TARGETS = {
    ("Darwin", "arm64"): {
        "key": "darwin-arm64",
        "asset": "syft_1.54.0_darwin_arm64.tar.gz",
        "sha256": "7e0bdad94c569fc6d5785c9a657bbae3d4c4e140ccb5eace3d0b5b6bc2b6dbcf",
        "binary_sha256": "835607cdfbdbfc59335b0beadeefc47aa6aab7d3b403c11cfa65627d92a27f61",
        "platform": "darwin/arm64",
        "verifier_lock": "syft-darwin-verifier.lock",
        "verifier_lock_sha256": "bc22323572381258237ff65529b55f37387a3bdfddf1ccf82305d41443ddacf2",
    },
    ("Linux", "x86_64"): {
        "key": "linux-amd64",
        "asset": "syft_1.54.0_linux_amd64.tar.gz",
        "sha256": "54a87372498168b2d033e876fd41fa4e8035b872699e525a57046e1f2f09c860",
        "binary_sha256": "d46a9a61a6ae3d367f0a03748c5e9c59253e586c4388ab26ddcacebc2efa0d92",  # Native Linux run 37327710820.
        "platform": "linux/amd64",
        "verifier_lock": "syft-linux-verifier.lock",
        "verifier_lock_sha256": "e8c2913539b2dc4260ef8611e1f21daa56efbdf8b55199c34882808aecd6acea",
    },
}
LOCK_DIR = Path(__file__).absolute().parent
MAX_CHECKSUM_BYTES = 64 * 1024
MAX_BUNDLE_BYTES = 2 * 1024 * 1024
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_LOG_BYTES = 2 * 1024 * 1024
MAX_MEMBER_COUNT = 16
MAX_MEMBER_BYTES = 96 * 1024 * 1024
MAX_EXPANDED_BYTES = 128 * 1024 * 1024
MAX_METADATA_BYTES = 64 * 1024
MAX_ARCHIVE_STREAM_BYTES = MAX_EXPANDED_BYTES + MAX_METADATA_BYTES * (MAX_MEMBER_COUNT + 1) + 1024 * (MAX_MEMBER_COUNT + 4)
MAX_LOCK_BYTES = 256 * 1024
MAX_JSON_BYTES = 1024 * 1024
NETWORK_STEP_TIMEOUT = 90
ARCHIVE_STEP_TIMEOUT = 300
COMMAND_TIMEOUT = 900
TOTAL_TIMEOUT = 1800
ALLOWED_MEMBERS = {"CHANGELOG.md", "LICENSE", "README.md", "syft"}
ALLOWED_DOWNLOAD_URLS = {CHECKSUM_URL, BUNDLE_URL} | {
    f"https://github.com/anchore/syft/releases/download/{RELEASE_TAG}/{target['asset']}"
    for target in TARGETS.values()
}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path, deadline: float | None = None) -> str:
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            if deadline is not None and time.monotonic() >= deadline:
                raise TimeoutError("installer total wall-clock deadline exceeded while hashing")
            digest.update(chunk)
    return digest.hexdigest()


def _open_directory_nofollow(path: Path) -> int:
    """Open a directory by walking from / with no-follow directory descriptors."""
    absolute = Path(os.path.abspath(path))
    if not absolute.is_absolute():
        raise ValueError("directory path must be absolute")
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
    current = os.open(os.sep, flags)
    try:
        for component in absolute.parts[1:]:
            next_fd = os.open(component, flags, dir_fd=current)
            os.close(current)
            current = next_fd
        return current
    except OSError as exc:
        os.close(current)
        raise RuntimeError("directory ancestry contains a symlink or inaccessible component") from exc


def _read_bounded_regular_nofollow(path: Path, byte_limit: int) -> bytes:
    """Read a bounded regular file through a stable no-follow parent descriptor."""
    absolute = Path(os.path.abspath(path))
    if not absolute.name:
        raise ValueError("file path has no leaf name")
    parent_fd = _open_directory_nofollow(absolute.parent)
    leaf_fd: int | None = None
    try:
        flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0) | getattr(os, "O_CLOEXEC", 0)
        try:
            leaf_fd = os.open(absolute.name, flags, dir_fd=parent_fd)
        except OSError as exc:
            raise RuntimeError("file must be a regular non-symlink file") from exc
        info = os.fstat(leaf_fd)
        if not stat.S_ISREG(info.st_mode) or info.st_size > byte_limit:
            raise RuntimeError("file is not a bounded regular file")
        chunks = bytearray()
        while True:
            block = os.read(leaf_fd, min(65536, byte_limit + 1 - len(chunks)))
            if not block:
                break
            chunks.extend(block)
            if len(chunks) > byte_limit:
                raise RuntimeError("file grew beyond its byte limit")
        return bytes(chunks)
    finally:
        if leaf_fd is not None:
            os.close(leaf_fd)
        os.close(parent_fd)


def _hash_bounded_regular_nofollow(path: Path, byte_limit: int) -> tuple[str, int]:
    absolute = Path(os.path.abspath(path))
    parent_fd = _open_directory_nofollow(absolute.parent)
    leaf_fd: int | None = None
    try:
        flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0) | getattr(os, "O_CLOEXEC", 0)
        try:
            leaf_fd = os.open(absolute.name, flags, dir_fd=parent_fd)
        except OSError as exc:
            raise RuntimeError("file must be a regular non-symlink file") from exc
        info = os.fstat(leaf_fd)
        if not stat.S_ISREG(info.st_mode) or info.st_size > byte_limit:
            raise RuntimeError("file is not a bounded regular file")
        digest = hashlib.sha256()
        count = 0
        while True:
            block = os.read(leaf_fd, min(1024 * 1024, byte_limit + 1 - count))
            if not block:
                break
            count += len(block)
            if count > byte_limit:
                raise RuntimeError("file grew beyond its byte limit")
            digest.update(block)
        return digest.hexdigest(), count
    finally:
        if leaf_fd is not None:
            os.close(leaf_fd)
        os.close(parent_fd)


def _remove_directory_contents(directory_fd: int) -> None:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
    for name in os.listdir(directory_fd):
        entry = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        if stat.S_ISDIR(entry.st_mode):
            child_fd = os.open(name, flags, dir_fd=directory_fd)
            try:
                opened = os.fstat(child_fd)
                if (opened.st_dev, opened.st_ino) != (entry.st_dev, entry.st_ino):
                    raise RuntimeError("output child directory changed during cleanup")
                _remove_directory_contents(child_fd)
            finally:
                os.close(child_fd)
            current = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            if (current.st_dev, current.st_ino) != (entry.st_dev, entry.st_ino):
                raise RuntimeError("output child directory changed during cleanup")
            os.rmdir(name, dir_fd=directory_fd)
        else:
            os.unlink(name, dir_fd=directory_fd)


def _remove_owned_directory(path: Path, identity: tuple[int, int] | None) -> bool:
    """Remove only the exact directory inode created by this invocation."""
    if identity is None:
        return False
    parent_fd = _open_directory_nofollow(path.parent)
    directory_fd: int | None = None
    try:
        flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
        directory_fd = os.open(path.name, flags, dir_fd=parent_fd)
        opened = os.fstat(directory_fd)
        entry = os.stat(path.name, dir_fd=parent_fd, follow_symlinks=False)
        if not stat.S_ISDIR(opened.st_mode) or (opened.st_dev, opened.st_ino) != identity or (entry.st_dev, entry.st_ino) != identity:
            return False
        _remove_directory_contents(directory_fd)
        os.close(directory_fd)
        directory_fd = None
        current = os.stat(path.name, dir_fd=parent_fd, follow_symlinks=False)
        if (current.st_dev, current.st_ino) != identity:
            return False
        os.rmdir(path.name, dir_fd=parent_fd)
        return True
    except (FileNotFoundError, NotADirectoryError, OSError, RuntimeError):
        return False
    finally:
        if directory_fd is not None:
            os.close(directory_fd)
        os.close(parent_fd)


def _scan_tar_gzip_bounds(archive_data: bytes, deadline_check: Callable[[], float] | None = None) -> None:
    """Bound decompressed tar bytes and metadata before tarfile processes headers."""
    expanded = 0
    raw_members = 0
    ended = False
    with gzip.GzipFile(fileobj=io.BytesIO(archive_data), mode="rb") as stream:
        def read_exact(size: int) -> bytes:
            nonlocal expanded
            chunks = bytearray()
            while len(chunks) < size:
                if deadline_check is not None:
                    deadline_check()
                block = stream.read(min(65536, size - len(chunks)))
                if not block:
                    raise ValueError("truncated gzip tar archive")
                expanded += len(block)
                if expanded > MAX_ARCHIVE_STREAM_BYTES:
                    raise ValueError("archive decompressed stream limit exceeded")
                chunks.extend(block)
            return bytes(chunks)

        def drain(size: int) -> None:
            remaining = size
            while remaining:
                amount = min(65536, remaining)
                read_exact(amount)
                remaining -= amount

        while True:
            header = read_exact(512)
            if header == b"\0" * 512:
                if read_exact(512) != b"\0" * 512:
                    raise ValueError("tar end marker is malformed")
                ended = True
                break
            raw_members += 1
            if raw_members > MAX_MEMBER_COUNT * 2 + 1:
                raise ValueError("archive raw member/header count exceeded")
            info = tarfile.TarInfo.frombuf(header, "utf-8", "surrogateescape")
            size = info.size
            if size < 0 or size > MAX_MEMBER_BYTES:
                raise ValueError("archive header declares an oversized member")
            if info.type in (tarfile.XHDTYPE, tarfile.XGLTYPE):
                if size > MAX_METADATA_BYTES:
                    raise ValueError("archive metadata header exceeds byte limit")
                metadata = read_exact(size)
                offset = 0
                while offset < len(metadata):
                    space = metadata.find(b" ", offset)
                    if space < 0 or not metadata[offset:space].isascii() or not metadata[offset:space].isdigit():
                        raise ValueError("archive metadata record is malformed")
                    record_size = int(metadata[offset:space])
                    end = offset + record_size
                    if record_size <= space - offset + 2 or end > len(metadata) or metadata[end - 1:end] != b"\n":
                        raise ValueError("archive metadata record length is invalid")
                    record = metadata[space + 1:end - 1]
                    key, separator, _value = record.partition(b"=")
                    if not separator or key == b"size" or key.startswith(b"GNU.sparse"):
                        raise ValueError("archive metadata contains a size or sparse override")
                    offset = end
            elif info.type in (tarfile.REGTYPE, tarfile.AREGTYPE):
                drain(size)
            else:
                raise ValueError("archive header is not a regular file or bounded PAX metadata")
            padding = (-size) % 512
            if padding:
                drain(padding)
        if not ended:
            raise ValueError("archive is missing end marker")
        while True:
            if deadline_check is not None:
                deadline_check()
            block = stream.read(65536)
            if not block:
                break
            expanded += len(block)
            if expanded > MAX_ARCHIVE_STREAM_BYTES:
                raise ValueError("archive decompressed stream limit exceeded")


def require_supported_python(version_info: Any = sys.version_info) -> None:
    actual = tuple(version_info[:3])
    if actual != (3, 14, 8):
        raise RuntimeError(f"Python 3.14.8 is required; found {'.'.join(map(str, actual))}")


def strict_json(data: bytes, *, label: str, byte_limit: int = MAX_JSON_BYTES, depth_limit: int = 32) -> Any:
    if len(data) > byte_limit:
        raise ValueError(f"{label} exceeds JSON byte limit")
    depth = 0
    in_string = False
    escaped = False
    for byte in data:
        if in_string:
            if escaped:
                escaped = False
            elif byte == 0x5C:
                escaped = True
            elif byte == 0x22:
                in_string = False
            continue
        if byte == 0x22:
            in_string = True
        elif byte in (0x7B, 0x5B):
            depth += 1
            if depth > depth_limit:
                raise ValueError(f"{label} exceeds JSON nesting limit")
        elif byte in (0x7D, 0x5D):
            depth -= 1
            if depth < 0:
                raise ValueError(f"{label} has invalid JSON structure")
    if in_string or depth != 0:
        raise ValueError(f"{label} has invalid JSON structure")

    def object_pairs(rows: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in rows:
            if key in result:
                raise ValueError(f"{label} contains a duplicate JSON key: {key}")
            result[key] = value
        return result

    def reject_constant(value: str) -> None:
        raise ValueError(f"{label} contains non-finite JSON value: {value}")

    def finite_float(value: str) -> float:
        number = float(value)
        if not math.isfinite(number):
            raise ValueError(f"{label} contains non-finite JSON number: {value}")
        return number

    return json.loads(data, object_pairs_hook=object_pairs, parse_constant=reject_constant, parse_float=finite_float)


def detect_target(system: str | None = None, machine: str | None = None) -> dict[str, Any]:
    actual_system = system or platform.system()
    actual_machine = (machine or platform.machine()).lower()
    if actual_machine == "aarch64" and actual_system == "Darwin":
        actual_machine = "arm64"
    if actual_machine == "amd64" and actual_system == "Linux":
        actual_machine = "x86_64"
    target = TARGETS.get((actual_system, actual_machine))
    if target is None:
        raise RuntimeError(f"unsupported host: {actual_system}/{actual_machine}")
    return dict(target)


def validate_lock(target: dict[str, Any], lock_dir: Path = LOCK_DIR) -> tuple[Path, bytes]:
    path = lock_dir / target["verifier_lock"]
    data = _read_bounded_regular_nofollow(path, MAX_LOCK_BYTES)
    if sha256_bytes(data) != target["verifier_lock_sha256"]:
        raise RuntimeError("verifier lock digest mismatch")
    text = data.decode("utf-8")
    rows: dict[str, tuple[str, str]] = {}
    for number, line in enumerate(text.splitlines(), 1):
        match = re.fullmatch(r"([A-Za-z0-9_.-]+)==([A-Za-z0-9.!+_-]+) --hash=sha256:([0-9a-f]{64})", line)
        if not match:
            raise RuntimeError(f"verifier lock line {number} is not a single pinned SHA-256 row")
        name, version, digest = match.groups()
        canonical = re.sub(r"[-_.]+", "-", name).lower()
        if canonical in rows:
            raise RuntimeError(f"duplicate verifier lock package: {name}")
        rows[canonical] = (version, digest)
    if rows.get("sigstore", (None, None))[0] != "4.5.0":
        raise RuntimeError("verifier lock must pin Sigstore 4.5.0")
    if rows.get("pypi-attestations", (None, None))[0] != "0.0.30":
        raise RuntimeError("verifier lock must retain the qualified verifier dependency closure")
    return path, data


def check_new_private_directory(path: Path) -> Path:
    path = Path(os.path.abspath(path))
    if not path.name:
        raise ValueError("output path must name a new directory")
    parent_fd = _open_directory_nofollow(path.parent)
    try:
        try:
            os.stat(path.name, dir_fd=parent_fd, follow_symlinks=False)
        except FileNotFoundError:
            pass
        else:
            raise FileExistsError(f"output already exists: {path}")
    finally:
        os.close(parent_fd)
    return path


def safe_write(path: Path, data: bytes, mode: int = 0o600) -> None:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    fd = os.open(path, flags, mode)
    with os.fdopen(fd, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def _group_exists(pgid: int) -> bool:
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def _stop_process_group(process: subprocess.Popen[bytes], grace_s: float = 0.5) -> None:
    pgid = process.pid
    try:
        os.killpg(pgid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    end = time.monotonic() + grace_s
    while time.monotonic() < end:
        process.poll()  # Reap the direct child so its zombie cannot pin the group.
        if not _group_exists(pgid):
            return
        time.sleep(0.02)
    try:
        os.killpg(pgid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    end = time.monotonic() + 2.0
    while time.monotonic() < end:
        process.poll()
        if not _group_exists(pgid):
            return
        time.sleep(0.02)
    process.poll()
    if _group_exists(pgid):
        raise RuntimeError("child process group survived bounded termination")


def run_bounded_command(
    argv: list[str], *, cwd: Path, env: dict[str, str], timeout: float,
    log_path: Path, max_output_bytes: int = MAX_LOG_BYTES,
) -> tuple[int, str, str]:
    """Run process group with bounded wall time and combined stdout/stderr logs."""
    import selectors

    if timeout <= 0:
        raise ValueError("command timeout must be positive")
    fd = os.open(log_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
    process: subprocess.Popen[bytes] | None = None
    selector = selectors.DefaultSelector()
    count = 0
    captured = {"stdout": bytearray(), "stderr": bytearray()}
    deadline = time.monotonic() + timeout
    try:
        with os.fdopen(fd, "wb") as log:
            process = subprocess.Popen(
                argv, cwd=str(cwd), env=env, stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0,
                start_new_session=True,
            )
            assert process.stdout is not None and process.stderr is not None
            selector.register(process.stdout, selectors.EVENT_READ, "stdout")
            selector.register(process.stderr, selectors.EVENT_READ, "stderr")
            while selector.get_map():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise subprocess.TimeoutExpired(argv, timeout)
                events = selector.select(min(remaining, 0.1))
                for key, _ in events:
                    remaining_cap = max_output_bytes - count
                    block = os.read(key.fileobj.fileno(), min(65536, remaining_cap + 1))
                    if not block:
                        selector.unregister(key.fileobj)
                        continue
                    if len(block) > remaining_cap:
                        prefix = block[:max(remaining_cap, 0)]
                        log.write(prefix)
                        count += len(prefix)
                        log.flush()
                        raise RuntimeError("child output exceeded the bounded log size")
                    log.write(block)
                    count += len(block)
                    captured[key.data].extend(block)
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise subprocess.TimeoutExpired(argv, timeout)
            returncode = process.wait(timeout=remaining)
            log.flush()
        return (returncode,
                captured["stdout"].decode("utf-8", errors="replace"),
                captured["stderr"].decode("utf-8", errors="replace"))
    except BaseException as original:
        if process is not None:
            try:
                _stop_process_group(process)
            except Exception as cleanup_error:
                raise RuntimeError(f"child cleanup failed after {type(original).__name__}: {cleanup_error}") from original
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired as exc:
                raise RuntimeError(f"child process was not reaped after {type(original).__name__}") from original
        raise
    finally:
        selector.close()
        if process is not None:
            for pipe in (process.stdout, process.stderr):
                if pipe is not None:
                    pipe.close()


def _checked_url(url: str) -> urllib.parse.SplitResult:
    parsed = urllib.parse.urlsplit(url)
    if (url not in ALLOWED_DOWNLOAD_URLS or parsed.scheme != "https" or parsed.hostname != "github.com"
            or parsed.port not in (None, 443) or parsed.username or parsed.password or parsed.fragment):
        raise RuntimeError("refusing unpinned GitHub release URL")
    return parsed


def validate_version_output(output: str, expected_platform: str) -> dict[str, Any]:
    try:
        value = strict_json(output.encode("utf-8"), label="Syft version output", byte_limit=MAX_LOG_BYTES)
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
        raise RuntimeError("Syft version probe is not strict JSON") from exc
    if (not isinstance(value, dict) or value.get("application") != "syft"
            or value.get("version") != VERSION or value.get("gitCommit") != RELEASE_COMMIT
            or value.get("platform") != expected_platform):
        raise RuntimeError("Syft version probe does not match pinned version, commit and platform")
    return value


class _ReleaseRedirectHandler(urllib.request.HTTPRedirectHandler):
    def __init__(self) -> None:
        super().__init__()
        self.redirect_count = 0

    def redirect_request(self, request, response, code, message, headers, new_url):
        old = urllib.parse.urlsplit(request.full_url)
        new = urllib.parse.urlsplit(new_url)
        old_host = old.hostname
        if (old.scheme != "https" or old_host not in {"github.com", "release-assets.githubusercontent.com"}
                or new.scheme != "https" or new.hostname not in {"github.com", "release-assets.githubusercontent.com"}
                or new.port not in (None, 443) or new.username or new.password or new.fragment):
            raise RuntimeError("refusing cross-origin or non-HTTPS release redirect")
        if old_host == new.hostname or (old_host == "github.com" and new.hostname == "release-assets.githubusercontent.com"):
            self.redirect_count += 1
            if self.redirect_count > 4:
                raise RuntimeError("release redirect limit exceeded")
            return super().redirect_request(request, response, code, message, headers, new_url)
        raise RuntimeError("refusing release redirect to an unapproved origin")


def _fetch_to_file(url: str, destination: Path, limit: int) -> None:
    """Child-process entrypoint; parent enforces a hard wall-clock deadline."""
    require_supported_python()
    _checked_url(url)
    handler = _ReleaseRedirectHandler()
    opener = urllib.request.build_opener(handler)
    request = urllib.request.Request(url, headers={"Accept": "application/octet-stream", "User-Agent": "kairos-verified-syft-installer/1"})
    with opener.open(request, timeout=15) as response:
        final = urllib.parse.urlsplit(response.geturl())
        if final.scheme != "https" or final.hostname not in {"github.com", "release-assets.githubusercontent.com"} or final.port not in (None, 443):
            raise RuntimeError("release response origin is not approved")
        if response.status != 200:
            raise RuntimeError(f"release response status {response.status}")
        content_type = response.headers.get("Content-Type", "").split(";", 1)[0].strip().lower()
        if content_type not in {"application/octet-stream", "application/json", "text/plain"}:
            raise RuntimeError("release response Content-Type is not an approved release-file type")
        header = response.headers.get("Content-Length")
        if header is not None and (not header.isdigit() or int(header) > limit):
            raise RuntimeError("release response Content-Length is invalid or over limit")
        data = bytearray()
        while True:
            block = response.read(min(65536, limit + 1 - len(data)))
            if not block:
                break
            data.extend(block)
            if len(data) > limit:
                raise RuntimeError("release response exceeded byte limit")
        if header is not None and int(header) != len(data):
            raise RuntimeError("release response length differs from Content-Length")
    safe_write(destination, bytes(data))


class Installer:
    def __init__(
        self, output: Path, *, execute: Callable[..., tuple[int, str]] = run_bounded_command,
        fetch: Callable[[str, Path, int], None] | None = None,
        system: str | None = None, machine: str | None = None,
        python: Path | None = None,
    ) -> None:
        self.output = check_new_private_directory(output)
        # Explicit host overrides exist only for isolated unit tests; CLI never supplies them.
        self.target = detect_target(system, machine)
        self.execute_process = execute
        self.fetch_override = fetch
        self.python = python or Path(sys.executable)
        self.started = time.monotonic()
        self.deadline = self.started + TOTAL_TIMEOUT
        self.commands: list[dict[str, Any]] = []
        self.events: list[str] = []
        self.created_output = False
        self.output_identity: tuple[int, int] | None = None

    def _assert_output_anchored(self) -> None:
        if self.output_identity is None:
            raise RuntimeError("private output directory has not been created")
        parent_fd = _open_directory_nofollow(self.output.parent)
        output_fd: int | None = None
        try:
            flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
            output_fd = os.open(self.output.name, flags, dir_fd=parent_fd)
            info = os.fstat(output_fd)
            entry = os.stat(self.output.name, dir_fd=parent_fd, follow_symlinks=False)
            if not stat.S_ISDIR(info.st_mode) or (info.st_dev, info.st_ino) != self.output_identity or (entry.st_dev, entry.st_ino) != self.output_identity:
                raise RuntimeError("private output directory changed during the run")
        except OSError as exc:
            raise RuntimeError("private output directory ancestry changed during the run") from exc
        finally:
            if output_fd is not None:
                os.close(output_fd)
            os.close(parent_fd)

    def _remaining(self, step_cap: float) -> float:
        remaining = min(step_cap, self.deadline - time.monotonic())
        if remaining <= 0:
            raise TimeoutError("installer total wall-clock deadline exceeded")
        return remaining

    def _minimal_env(self) -> dict[str, str]:
        # HTTPS uses Python's default verified TLS context. Ambient proxy and
        # CA override variables are intentionally absent from every child.
        env = {"PATH": "/usr/bin:/bin", "LANG": "C", "LC_ALL": "C"}
        env.update({
            "HOME": str(self.output / "home"), "TMPDIR": str(self.output / "tmp"),
            "XDG_CACHE_HOME": str(self.output / "cache"), "XDG_CONFIG_HOME": str(self.output / "config"),
            "PIP_CACHE_DIR": str(self.output / "pip-cache"), "PIP_CONFIG_FILE": os.devnull,
            "PYTHONDONTWRITEBYTECODE": "1", "PYTHONNOUSERSITE": "1",
        })
        return env

    def _run(self, label: str, argv: list[str], timeout: float = COMMAND_TIMEOUT) -> tuple[int, str, str]:
        self._assert_output_anchored()
        log = self.output / "logs" / f"{len(self.commands):02d}-{label}.log"
        started = time.monotonic()
        try:
            status, stdout, stderr = self.execute_process(
                argv, cwd=self.output, env=self._minimal_env(), timeout=self._remaining(timeout),
                log_path=log, max_output_bytes=MAX_LOG_BYTES,
            )
            self._assert_output_anchored()
            log_data = _read_bounded_regular_nofollow(log, MAX_LOG_BYTES)
            record = {"label": label, "argv": argv, "exit_status": status,
                      "elapsed_seconds": round(time.monotonic() - started, 3),
                      "log_sha256": sha256_bytes(log_data), "log_bytes": len(log_data),
                      "stdout_sha256": sha256_bytes(stdout.encode()),
                      "stderr_sha256": sha256_bytes(stderr.encode())}
            self.commands.append(record)
            if status != 0:
                raise RuntimeError(f"{label} failed with exit status {status}")
            return status, stdout, stderr
        except BaseException as exc:
            if log.is_file() and not any(c.get("label") == label for c in self.commands):
                self.commands.append({"label": label, "argv": argv, "exit_status": None,
                                      "error": type(exc).__name__, "elapsed_seconds": round(time.monotonic() - started, 3),
                                      "log_sha256": sha256_bytes(_read_bounded_regular_nofollow(log, MAX_LOG_BYTES)),
                                      "log_bytes": len(_read_bounded_regular_nofollow(log, MAX_LOG_BYTES))})
            raise

    def _make_output(self) -> None:
        parent_fd = _open_directory_nofollow(self.output.parent)
        output_fd: int | None = None
        try:
            os.mkdir(self.output.name, mode=0o700, dir_fd=parent_fd)
            self.created_output = True
            flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
            output_fd = os.open(self.output.name, flags, dir_fd=parent_fd)
            info = os.fstat(output_fd)
            entry = os.stat(self.output.name, dir_fd=parent_fd, follow_symlinks=False)
            if not stat.S_ISDIR(info.st_mode) or (info.st_dev, info.st_ino) != (entry.st_dev, entry.st_ino):
                raise RuntimeError("private output directory changed during creation")
            self.output_identity = (info.st_dev, info.st_ino)
            for name in ("home", "tmp", "cache", "config", "pip-cache", "logs", "downloads", "bin", "evidence"):
                os.mkdir(name, mode=0o700, dir_fd=output_fd)
        finally:
            if output_fd is not None:
                os.close(output_fd)
            os.close(parent_fd)
        self.created_output = True
        self._assert_output_anchored()
        self.events.append("created-fresh-private-output")

    def _download(self, label: str, url: str, filename: str, limit: int) -> Path:
        _checked_url(url)
        self._assert_output_anchored()
        destination = self.output / "downloads" / filename
        if self.fetch_override is not None:
            self.fetch_override(url, destination, limit)
        else:
            self._run(label, [str(self.python), str(Path(__file__).resolve()), "--fetch-internal", url, str(destination), str(limit)], NETWORK_STEP_TIMEOUT)
        _hash_bounded_regular_nofollow(destination, limit)
        self._assert_output_anchored()
        return destination

    def _install_verifier(self, lock_path: Path) -> Path:
        venv = self.output / "verifier"
        self._run("create-verifier-venv", [str(self.python), "-m", "venv", str(venv)], 120)
        verifier_python = venv / "bin" / "python"
        if not verifier_python.is_file():
            raise RuntimeError("verifier Python executable missing")
        probe = self.output / "pip-config-audit.py"
        source = '''import os, pathlib, sys\nfrom pip._internal import configuration as module\nfrom pip._internal.configuration import Configuration, kinds\nif os.environ.get("PIP_CONFIG_FILE") != os.devnull:\n    raise SystemExit("PIP_CONFIG_FILE is not the null device")\nfixture = pathlib.Path(sys.argv[1])\nfixture.write_text("[global]\\nextra-index-url = https://config-canary.invalid/simple\\nfind-links = https://config-canary.invalid/wheels\\ntrusted-host = config-canary.invalid\\n", encoding="utf-8")\nmodule.get_configuration_files = lambda: {kind: [str(fixture)] for kind in (kinds.GLOBAL, kinds.USER, kinds.SITE)}\nconfig = Configuration(isolated=True)\nconfig.load()\nif any(config.get_values_in_config(kind) for kind in (kinds.GLOBAL, kinds.USER, kinds.SITE, kinds.ENV, kinds.ENV_VAR)):\n    raise SystemExit("pip loaded an uncontrolled configuration source")\nprint("isolated pip configuration audit passed")\n'''
        self._assert_output_anchored()
        safe_write(probe, source.encode("utf-8"))
        self._run("audit-pip-configuration", [str(verifier_python), str(probe), str(self.output / "pip-canary.ini")], 30)
        self._run("install-hash-locked-verifier", [str(verifier_python), "-m", "pip", "--isolated", "--disable-pip-version-check", "--no-input", "install", "--require-hashes", "-r", str(lock_path)], 900)
        self.events.append("installed-hash-locked-sigstore-4.5.0")
        return verifier_python

    @staticmethod
    def _parse_checksum(data: bytes, asset: str) -> str:
        try:
            text = data.decode("ascii")
        except UnicodeDecodeError as exc:
            raise ValueError("signed checksum file is not ASCII") from exc
        rows = []
        for line in text.splitlines():
            match = re.fullmatch(r"([0-9a-f]{64})[ \t]+\*?([A-Za-z0-9_.+-]+)", line)
            if not match:
                raise ValueError("signed checksum file contains malformed row")
            if match.group(2) == asset:
                rows.append(match.group(1))
        if len(rows) != 1:
            raise ValueError("signed checksum file must contain exactly one target row")
        return rows[0]

    def _verify_checksums(self, verifier: Path, checksums: Path, bundle: Path) -> None:
        # Sigstore 4.5.0's `verify github` implementation unconditionally applies
        # OIDCIssuer(https://token.actions.githubusercontent.com); there is no
        # --cert-oidc-issuer option on that subcommand. Keep that exact CLI policy.
        args = [str(verifier), "-m", "sigstore", "verify", "github", "--bundle", str(bundle),
                "--cert-identity", CERT_IDENTITY, "--sha", RELEASE_COMMIT,
                "--repository", REPOSITORY, "--ref", WORKFLOW_REF, str(checksums)]
        self._run("verify-signed-checksum-document", args, 300)
        self.events.append("sigstore-github-identity-repository-ref-sha-and-hardcoded-issuer-verified")

    def _safe_extract(self, archive_path: Path, destination: Path, expected_archive_sha256: str | None = None) -> tuple[Path, dict[str, Any]]:
        archive_data = _read_bounded_regular_nofollow(archive_path, MAX_ARCHIVE_BYTES)
        archive_sha256 = sha256_bytes(archive_data)
        if expected_archive_sha256 is not None and archive_sha256 != expected_archive_sha256:
            raise ValueError("archive changed after its authenticated digest check")
        _scan_tar_gzip_bounds(archive_data, lambda: self._remaining(TOTAL_TIMEOUT))
        destination = Path(os.path.abspath(destination))
        if not destination.name:
            raise ValueError("binary extraction destination must have a leaf name")
        parent_fd = _open_directory_nofollow(destination.parent)
        destination_fd: int | None = None
        created_destination = False
        created_names: list[str] = []
        try:
            try:
                os.stat(destination.name, dir_fd=parent_fd, follow_symlinks=False)
            except FileNotFoundError:
                pass
            else:
                raise FileExistsError("binary extraction destination already exists")
            os.mkdir(destination.name, mode=0o700, dir_fd=parent_fd)
            created_destination = True
            dir_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
            destination_fd = os.open(destination.name, dir_flags, dir_fd=parent_fd)
            opened = os.fstat(destination_fd)
            current = os.stat(destination.name, dir_fd=parent_fd, follow_symlinks=False)
            if (opened.st_dev, opened.st_ino) != (current.st_dev, current.st_ino):
                raise RuntimeError("extraction destination changed during creation")
            seen: set[str] = set()
            total = 0
            count = 0
            extracted: dict[str, str] = {}
            with tarfile.open(fileobj=io.BytesIO(archive_data), mode="r:gz") as archive:
                for member in archive:
                    self._remaining(TOTAL_TIMEOUT)
                    count += 1
                    if count > MAX_MEMBER_COUNT:
                        raise ValueError("archive member count exceeded")
                    name = member.name
                    pure = PurePosixPath(name)
                    if (not name or "\\" in name or pure.is_absolute() or any(part in {"", ".", ".."} for part in name.split("/"))
                            or re.match(r"^[A-Za-z]:", name) or name not in ALLOWED_MEMBERS):
                        raise ValueError(f"unsafe or unexpected archive path: {name!r}")
                    canonical = pure.as_posix()
                    if canonical in seen:
                        raise ValueError(f"duplicate canonical archive path: {canonical}")
                    seen.add(canonical)
                    if not member.isreg() or member.size < 0 or member.size > MAX_MEMBER_BYTES:
                        raise ValueError(f"unsafe or oversized archive member: {name}")
                    total += member.size
                    if total > MAX_EXPANDED_BYTES:
                        raise ValueError("archive expanded-byte limit exceeded")
                    source = archive.extractfile(member)
                    if source is None:
                        raise ValueError(f"archive member cannot be read: {name}")
                    digest = hashlib.sha256()
                    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
                    fd = os.open(name, flags, 0o755 if name == "syft" else 0o600, dir_fd=destination_fd)
                    created_names.append(name)
                    copied = 0
                    try:
                        with os.fdopen(fd, "wb") as output, source:
                            while True:
                                self._remaining(TOTAL_TIMEOUT)
                                block = source.read(min(1024 * 1024, member.size - copied + 1))
                                if not block:
                                    break
                                copied += len(block)
                                if copied > member.size:
                                    raise ValueError(f"member exceeded declared size: {name}")
                                digest.update(block)
                                output.write(block)
                            if copied != member.size:
                                raise ValueError(f"member size mismatch: {name}")
                            output.flush()
                            os.fsync(output.fileno())
                    except BaseException:
                        raise
                    extracted[name] = digest.hexdigest()
            if set(extracted) != ALLOWED_MEMBERS:
                raise ValueError("archive members do not match the exact reviewed Syft profile")
            return destination / "syft", {"member_count": count, "expanded_bytes": total,
                                          "members": extracted, "binary_sha256": extracted["syft"],
                                          "archive_sha256": archive_sha256,
                                          "decompressed_stream_limit": MAX_ARCHIVE_STREAM_BYTES,
                                          "metadata_header_limit": MAX_METADATA_BYTES}
        except BaseException:
            if destination_fd is not None:
                for name in created_names:
                    try:
                        os.unlink(name, dir_fd=destination_fd)
                    except FileNotFoundError:
                        pass
                os.close(destination_fd)
                destination_fd = None
            if created_destination:
                try:
                    os.rmdir(destination.name, dir_fd=parent_fd)
                except FileNotFoundError:
                    pass
            raise
        finally:
            if destination_fd is not None:
                os.close(destination_fd)
            os.close(parent_fd)

    def run(self) -> Path:
        require_supported_python()
        lock_path, lock_bytes = validate_lock(self.target)
        try:
            self._make_output()
            retained_lock = self.output / "verifier.lock"
            self._assert_output_anchored()
            safe_write(retained_lock, lock_bytes)
            if _hash_bounded_regular_nofollow(retained_lock, MAX_LOCK_BYTES)[0] != self.target["verifier_lock_sha256"]:
                raise RuntimeError("private verifier lock copy digest mismatch")
            checksums = self._download("download-checksums", CHECKSUM_URL, "syft-checksums.txt", MAX_CHECKSUM_BYTES)
            bundle = self._download("download-signature-bundle", BUNDLE_URL, "syft-checksums.sigstore.json", MAX_BUNDLE_BYTES)
            checksum_digest, _ = _hash_bounded_regular_nofollow(checksums, MAX_CHECKSUM_BYTES)
            bundle_digest, _ = _hash_bounded_regular_nofollow(bundle, MAX_BUNDLE_BYTES)
            if checksum_digest != CHECKSUM_SHA256 or bundle_digest != BUNDLE_SHA256:
                raise RuntimeError("retained checksum or bundle digest mismatch")
            verifier = self._install_verifier(retained_lock)
            self._verify_checksums(verifier, checksums, bundle)
            checksum_bytes = _read_bounded_regular_nofollow(checksums, MAX_CHECKSUM_BYTES)
            signed_digest = self._parse_checksum(checksum_bytes, self.target["asset"])
            if signed_digest != self.target["sha256"]:
                raise RuntimeError("authenticated target checksum differs from the retained release row")
            archive_url = f"https://github.com/anchore/syft/releases/download/{RELEASE_TAG}/{self.target['asset']}"
            archive = self._download("download-syft-archive", archive_url, self.target["asset"], MAX_ARCHIVE_BYTES)
            archive_digest, _ = _hash_bounded_regular_nofollow(archive, MAX_ARCHIVE_BYTES)
            if archive_digest != signed_digest:
                raise RuntimeError("downloaded Syft archive digest does not match authenticated checksum")
            _, extraction_output, _ = self._run("extract-syft-archive", [str(self.python), str(Path(__file__).absolute()),
                                                                          "--extract-internal", str(archive),
                                                                          str(self.output / "extract"), archive_digest],
                                                ARCHIVE_STEP_TIMEOUT)
            extracted = strict_json(extraction_output.encode("utf-8"), label="archive extraction report",
                                    byte_limit=64 * 1024)
            if not isinstance(extracted, dict) or not isinstance(extracted.get("archive"), dict):
                raise RuntimeError("bounded archive extractor returned an invalid report")
            archive_receipt = extracted["archive"]
            binary = self.output / "extract" / "syft"
            binary_digest, _ = _hash_bounded_regular_nofollow(binary, MAX_MEMBER_BYTES)
            if binary_digest != archive_receipt.get("binary_sha256"):
                raise RuntimeError("extracted Syft binary changed after bounded extraction")
            if archive_receipt.get("archive_sha256") != archive_digest:
                raise RuntimeError("bounded archive extractor reported a different source digest")
            if self.target["binary_sha256"] and archive_receipt["binary_sha256"] != self.target["binary_sha256"]:
                raise RuntimeError("extracted Syft binary digest differs from retained qualification")
            final_binary = self.output / "bin" / "syft"
            self._assert_output_anchored()
            extract_fd = _open_directory_nofollow(binary.parent)
            try:
                extract_info = os.fstat(extract_fd)
                extract_identity = (extract_info.st_dev, extract_info.st_ino)
            finally:
                os.close(extract_fd)
            os.replace(binary, final_binary)
            if not _remove_owned_directory(binary.parent, extract_identity):
                raise RuntimeError("cannot safely remove the completed extraction directory")
            _status, output, _stderr = self._run("syft-version", [str(final_binary), "version", "-o", "json"], 60)
            value = validate_version_output(output, self.target["platform"])
            python_path = Path(sys.executable).resolve(strict=True)
            python_info = {
                "version": sys.version,
                "implementation": platform.python_implementation(),
                "executable": str(python_path),
                "executable_sha256": sha256_file(python_path, self.deadline),
                "tls_default_verify_paths": {
                    key: value for key, value in ssl.get_default_verify_paths()._asdict().items()
                    if key in {"cafile", "capath", "openssl_cafile", "openssl_capath"}
                },
            }
            receipt = {
                "schema": "kairos.verified-syft-installer.v1", "result": "pass",
                "version": VERSION, "release_tag": RELEASE_TAG, "release_commit": RELEASE_COMMIT,
                "target": self.target["key"], "platform": self.target["platform"],
                "release_checksum_sha256": checksum_digest, "release_bundle_sha256": bundle_digest,
                "authenticated_asset": self.target["asset"], "signed_asset_sha256": signed_digest,
                "archive_sha256": archive_digest, "binary_path": "bin/syft",
                "binary_sha256": _hash_bounded_regular_nofollow(final_binary, MAX_MEMBER_BYTES)[0], "archive": archive_receipt,
                "version_probe": value, "issuer_policy": OIDC_ISSUER,
                "issuer_enforcement": "Sigstore 4.5.0 verify github applies OIDCIssuer unconditionally; subcommand has no issuer flag",
                "certificate_identity": CERT_IDENTITY, "repository": REPOSITORY, "ref": WORKFLOW_REF,
                "installer_source_sha256": sha256_file(Path(__file__).resolve(), self.deadline),
                "python_toolchain": python_info,
                "verifier": {"sigstore": "4.5.0", "lock_path": self.target["verifier_lock"],
                             "lock_sha256": sha256_bytes(lock_bytes)},
                "commands": self.commands, "events": self.events,
                "qualification_limit": "Native authenticated installation and version probe only; no package scan, release, or publication is represented.",
            }
            receipt_path = self.output / "evidence" / "receipt.json"
            self._assert_output_anchored()
            safe_write(receipt_path, (json.dumps(receipt, sort_keys=True, indent=2) + "\n").encode())
            return final_binary
        except BaseException:
            if self.created_output:
                _remove_owned_directory(self.output, self.output_identity)
            raise


def _internal_extract(argv: list[str]) -> int:
    if len(argv) != 3:
        raise ValueError("invalid internal extraction arguments")
    require_supported_python()
    archive_path, destination, expected_digest = argv
    target = detect_target()
    if Path(archive_path).name != target["asset"] or expected_digest != target["sha256"]:
        raise ValueError("internal extraction accepts only the pinned native-host archive and digest")
    extractor = object.__new__(Installer)
    extractor.deadline = time.monotonic() + TOTAL_TIMEOUT
    _binary, receipt = extractor._safe_extract(Path(archive_path), Path(destination), expected_digest)
    sys.stdout.write(json.dumps({"binary": "syft", "archive": receipt}, sort_keys=True) + "\n")
    return 0


def _internal_fetch(argv: list[str]) -> int:
    require_supported_python()
    if len(argv) != 3:
        raise ValueError("invalid internal fetch arguments")
    url, path_text, limit_text = argv
    path = Path(path_text)
    limit = int(limit_text)
    if limit <= 0 or limit > MAX_ARCHIVE_BYTES:
        raise ValueError("invalid internal fetch size cap")
    _fetch_to_file(url, path, limit)
    return 0


def main(argv: list[str] | None = None) -> int:
    argsv = list(sys.argv[1:] if argv is None else argv)
    if argsv and argsv[0] == "--extract-internal":
        try:
            return _internal_extract(argsv[1:])
        except Exception as exc:
            print(f"bounded archive extraction failed: {type(exc).__name__}: {str(exc)[:300]}", file=sys.stderr)
            return 1
    if argsv and argsv[0] == "--fetch-internal":
        try:
            return _internal_fetch(argsv[1:])
        except Exception as exc:
            print(f"bounded release download failed: {type(exc).__name__}: {str(exc)[:300]}", file=sys.stderr)
            return 1
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True, help="new, private output directory; must not exist")
    args = parser.parse_args(argsv)
    try:
        installer = Installer(args.output_dir)
        binary = installer.run()
    except Exception as exc:
        print(f"verified Syft installation failed: {type(exc).__name__}: {str(exc)[:400]}", file=sys.stderr)
        return 1
    print(binary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

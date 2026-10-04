#!/usr/bin/env python3
"""Install the authenticated Syft Linux amd64 release into a fresh path."""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import selectors
import shutil
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
from datetime import datetime, timezone
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener
import uuid


VERSION = "1.54.0"
GIT_COMMIT = "cc326e45a6213360266dda4b30cc68095946d676"
ARCHIVE_NAME = f"syft_{VERSION}_linux_amd64.tar.gz"
CHECKSUM_NAME = f"syft_{VERSION}_checksums.txt"
BUNDLE_NAME = CHECKSUM_NAME + ".sigstore.json"
RELEASE_URL = f"https://github.com/anchore/syft/releases/download/v{VERSION}"
ARCHIVE_URL = f"{RELEASE_URL}/{ARCHIVE_NAME}"
CHECKSUM_URL = f"{RELEASE_URL}/{CHECKSUM_NAME}"
BUNDLE_URL = f"{RELEASE_URL}/{BUNDLE_NAME}"
ARCHIVE_SIZE = 29_217_540
ARCHIVE_SHA256 = "54a87372498168b2d033e876fd41fa4e8035b872699e525a57046e1f2f09c860"
CHECKSUM_SIZE = 2_690
CHECKSUM_SHA256 = "e423344e663d7d14db62e51ddd31e4a5012818ed69e78358391dc487483f839c"
BUNDLE_SIZE = 10_306
BUNDLE_SHA256 = "6a0dbf94cb89e2fb157f022bed752b3ceb5827cb557a4a4cfb6af67f98b99811"
SYFT_SHA256 = "d46a9a61a6ae3d367f0a03748c5e9c59253e586c4388ab26ddcacebc2efa0d92"
WORKFLOW_IDENTITY = "https://github.com/anchore/syft/.github/workflows/release.yaml@refs/heads/main"
OIDC_ISSUER = "https://token.actions.githubusercontent.com"
WORKFLOW_REF = "refs/heads/main"
ALLOW_REDIRECT_HOSTS = frozenset({
    "github.com", "release-assets.githubusercontent.com", "objects.githubusercontent.com",
})
READ_TIMEOUT_SECONDS = 15
ASSET_DEADLINE_SECONDS = 120
CHILD_TIMEOUT_SECONDS = 30
CHILD_OUTPUT_LIMIT = 1024 * 1024
CHILD_KILL_DRAIN_SECONDS = 2
READ_CHUNK_BYTES = 64 * 1024
TAR_METADATA_AND_PADDING_LIMIT = 64 * 1024


@dataclass(frozen=True)
class MemberPin:
    name: str
    size: int
    sha256: str | None = None


@dataclass(frozen=True)
class ReleasePins:
    version: str
    git_commit: str
    archive_name: str
    archive_url: str
    archive_size: int
    archive_sha256: str
    checksum_name: str
    checksum_url: str
    checksum_size: int
    checksum_sha256: str
    bundle_name: str
    bundle_url: str
    bundle_size: int
    bundle_sha256: str
    binary_sha256: str
    members: tuple[MemberPin, ...]


SYFT_LINUX_AMD64 = ReleasePins(
    version=VERSION,
    git_commit=GIT_COMMIT,
    archive_name=ARCHIVE_NAME,
    archive_url=ARCHIVE_URL,
    archive_size=ARCHIVE_SIZE,
    archive_sha256=ARCHIVE_SHA256,
    checksum_name=CHECKSUM_NAME,
    checksum_url=CHECKSUM_URL,
    checksum_size=CHECKSUM_SIZE,
    checksum_sha256=CHECKSUM_SHA256,
    bundle_name=BUNDLE_NAME,
    bundle_url=BUNDLE_URL,
    bundle_size=BUNDLE_SIZE,
    bundle_sha256=BUNDLE_SHA256,
    binary_sha256=SYFT_SHA256,
    members=(
        MemberPin("CHANGELOG.md", 9_107),
        MemberPin("LICENSE", 11_357),
        MemberPin("README.md", 6_171),
        MemberPin("syft", 87_204_002, SYFT_SHA256),
    ),
)


class InstallError(RuntimeError):
    pass


class _AllowedRedirects(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        parsed = urlsplit(newurl)
        if parsed.scheme != "https" or parsed.hostname not in ALLOW_REDIRECT_HOSTS:
            raise InstallError("release download redirected outside the HTTPS allowlist")
        return super().redirect_request(req, fp, code, msg, headers, newurl)


class _BoundedReader:
    """Cap decompressed tar bytes, including headers and extension metadata."""

    def __init__(self, stream, limit: int):
        self.stream = stream
        self.limit = limit
        self.total = 0

    def read(self, size=-1):
        if size is None or size < 0:
            size = self.limit - self.total + 1
        data = self.stream.read(min(size, self.limit - self.total + 1))
        self.total += len(data)
        if self.total > self.limit:
            raise InstallError("decompressed tar stream exceeds ceiling")
        return data


def _hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(READ_CHUNK_BYTES), b""):
            digest.update(block)
    return digest.hexdigest()


def _safe_absolute_path(value: str | Path, label: str) -> Path:
    path = Path(value)
    if not path.is_absolute():
        raise InstallError(f"{label} must be absolute")
    try:
        resolved = path.resolve(strict=False)
    except OSError as exc:
        raise InstallError(f"cannot resolve {label}") from exc
    if resolved != path:
        raise InstallError(f"{label} contains a symlink or noncanonical parent")
    return path


def _safe_existing_parent(path: Path, label: str) -> None:
    parent = path.parent
    if not parent.is_dir():
        raise InstallError(f"{label} parent must already exist")
    current = Path(parent.anchor)
    for component in parent.parts[1:]:
        current = current / component
        if current.is_symlink() or not current.is_dir():
            raise InstallError(f"{label} parent has a symlink or non-directory component")


def _write_json(path: Path, value: dict) -> None:
    data = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")
    temp = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    fd = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp, path)
    except BaseException:
        temp.unlink(missing_ok=True)
        raise


def _record(receipt_path: Path, receipt: dict, stage: dict) -> None:
    receipt["stages"].append(stage)
    _write_json(receipt_path, receipt)


def _default_open(request: Request, timeout: int):
    return build_opener(_AllowedRedirects()).open(request, timeout=timeout)


def _download(
    url: str,
    destination: Path,
    expected_size: int,
    expected_sha256: str,
    *,
    open_url=_default_open,
    monotonic=time.monotonic,
    read_timeout: int = READ_TIMEOUT_SECONDS,
    deadline_seconds: int = ASSET_DEADLINE_SECONDS,
) -> dict:
    parsed = urlsplit(url)
    if parsed.scheme != "https" or parsed.hostname not in ALLOW_REDIRECT_HOSTS:
        raise InstallError("release asset URL is outside the HTTPS allowlist")
    if type(expected_size) is not int or expected_size <= 0 or not re.fullmatch(r"[0-9a-f]{64}", expected_sha256):
        raise InstallError("invalid pinned download identity")
    if type(read_timeout) is not int or read_timeout <= 0 or type(deadline_seconds) is not int or deadline_seconds <= 0:
        raise InstallError("invalid network timeout")
    request = Request(url, headers={"Accept-Encoding": "identity", "User-Agent": "kairos-syft-installer/1"})
    started = monotonic()
    digest = hashlib.sha256()
    total = 0
    created = False
    try:
        with destination.open("xb") as output:
            created = True
            response = open_url(request, timeout=read_timeout)
            with response:
                final = urlsplit(response.geturl())
                if final.scheme != "https" or final.hostname not in ALLOW_REDIRECT_HOSTS:
                    raise InstallError("release response URL is outside the HTTPS allowlist")
                headers = getattr(response, "headers", {})
                encoding = headers.get("Content-Encoding") if hasattr(headers, "get") else None
                if encoding not in (None, "", "identity"):
                    raise InstallError("unexpected content encoding")
                declared = headers.get("Content-Length") if hasattr(headers, "get") else None
                if declared is not None:
                    try:
                        declared_size = int(declared)
                    except (TypeError, ValueError) as exc:
                        raise InstallError("malformed Content-Length") from exc
                    if declared_size != expected_size:
                        raise InstallError("Content-Length differs from pinned size")
                while True:
                    if monotonic() - started > deadline_seconds:
                        raise InstallError("asset download exceeded monotonic deadline")
                    block = response.read(min(READ_CHUNK_BYTES, expected_size - total + 1))
                    if monotonic() - started > deadline_seconds:
                        raise InstallError("asset download exceeded monotonic deadline")
                    if not block:
                        break
                    if len(block) > expected_size - total:
                        raise InstallError("asset download exceeds pinned size")
                    output.write(block)
                    digest.update(block)
                    total += len(block)
            if total != expected_size:
                raise InstallError("asset download is truncated")
            actual_sha = digest.hexdigest()
            if actual_sha != expected_sha256:
                raise InstallError("asset SHA-256 differs from pin")
            output.flush()
            os.fsync(output.fileno())
        return {"url": url, "bytes": total, "sha256": actual_sha}
    except BaseException:
        if created:
            destination.unlink(missing_ok=True)
        raise


def _extract_archive(archive: Path, destination: Path, pins: ReleasePins) -> Path:
    if destination.exists():
        raise InstallError("extraction destination already exists")
    destination.mkdir(mode=0o700)
    expected = {member.name: member for member in pins.members}
    seen: set[str] = set()
    total = 0
    decompressed_limit = sum(item.size for item in pins.members) + TAR_METADATA_AND_PADDING_LIMIT
    binary = destination / "syft"
    source = None
    try:
        import gzip
        with archive.open("rb") as compressed, gzip.GzipFile(fileobj=compressed, mode="rb") as decompressed:
            bounded = _BoundedReader(decompressed, decompressed_limit)
            source = tarfile.open(fileobj=bounded, mode="r|")
            try:
                for member in source:
                    name = member.name
                    pure = PurePosixPath(name)
                    if (not name or pure.is_absolute() or ".." in pure.parts or "\\" in name
                            or ":" in name or pure.as_posix() != name):
                        raise InstallError("unsafe tar member path")
                    if name not in expected or name in seen:
                        raise InstallError("unexpected or duplicate tar member")
                    pin = expected[name]
                    if not member.isfile() or member.size != pin.size:
                        raise InstallError("tar member type or size differs from pin")
                    if member.pax_headers:
                        raise InstallError("tar extended metadata is not allowed")
                    total += member.size
                    if total > sum(item.size for item in pins.members):
                        raise InstallError("decompressed archive exceeds size ceiling")
                    src = source.extractfile(member)
                    if src is None:
                        raise InstallError("tar member has no data stream")
                    target = destination / name
                    digest = hashlib.sha256()
                    remaining = pin.size
                    with src, target.open("xb") as output:
                        while remaining:
                            block = src.read(min(READ_CHUNK_BYTES, remaining))
                            if not block:
                                raise InstallError("truncated tar member")
                            output.write(block)
                            digest.update(block)
                            remaining -= len(block)
                        if src.read(1):
                            raise InstallError("tar member exceeds pinned size")
                        output.flush()
                        os.fsync(output.fileno())
                    if pin.sha256 is not None and digest.hexdigest() != pin.sha256:
                        raise InstallError("extracted Syft binary SHA-256 differs from pin")
                    seen.add(name)
            finally:
                source.close()
                source = None
        if seen != set(expected):
            raise InstallError("tar archive is missing pinned members")
        binary.chmod(0o700)
        return binary
    except BaseException as exc:
        if source is not None:
            source.close()
        shutil.rmtree(destination, ignore_errors=True)
        if isinstance(exc, (tarfile.TarError, EOFError, OSError)):
            raise InstallError("invalid or truncated gzip tar archive") from exc
        raise


def _cosign_identity(path: str, expected_sha256: str) -> tuple[Path, os.stat_result]:
    if not re.fullmatch(r"[0-9a-f]{64}", expected_sha256):
        raise InstallError("Cosign SHA-256 must be 64 lowercase hexadecimal characters")
    candidate = _safe_absolute_path(path, "Cosign path")
    if candidate.is_symlink():
        raise InstallError("Cosign path must not be a symlink")
    try:
        resolved = candidate.resolve(strict=True)
        info = resolved.stat()
    except OSError as exc:
        raise InstallError("Cosign executable is unavailable") from exc
    if not stat.S_ISREG(info.st_mode) or not os.access(resolved, os.X_OK):
        raise InstallError("Cosign path must be a regular executable")
    if _hash_file(resolved) != expected_sha256:
        raise InstallError("Cosign executable SHA-256 differs from trusted setup input")
    return resolved, info


def _verify_cosign_identity(path: Path, expected_sha256: str, prior: os.stat_result) -> None:
    try:
        current = path.stat()
        actual = _hash_file(path)
    except OSError as exc:
        raise InstallError("Cosign executable disappeared") from exc
    if (current.st_dev, current.st_ino) != (prior.st_dev, prior.st_ino) or actual != expected_sha256:
        raise InstallError("Cosign executable identity changed during use")


def _command_environment(temp_dir: Path) -> dict[str, str]:
    env = {key: os.environ[key] for key in ("PATH", "HOME", "LANG", "LC_ALL") if key in os.environ}
    env["TMPDIR"] = str(temp_dir)
    return env


def _run_bounded_child(
    executable: Path,
    args: list[str],
    stage: str,
    evidence_dir: Path,
    env: dict[str, str],
    *,
    timeout_seconds: int = CHILD_TIMEOUT_SECONDS,
    output_limit: int = CHILD_OUTPUT_LIMIT,
    popen=subprocess.Popen,
    monotonic=time.monotonic,
) -> tuple[int, bytes, bytes]:
    argv = [str(executable), *args]
    stdout_parts: list[bytes] = []
    stderr_parts: list[bytes] = []
    counts = {"stdout": 0, "stderr": 0}
    parts = {"stdout": stdout_parts, "stderr": stderr_parts}
    proc = None
    selector = selectors.DefaultSelector()
    started = monotonic()
    started_utc = datetime.now(timezone.utc).isoformat()
    working_directory = os.getcwd()
    disposition = "exited"
    failure = None
    code = None
    pipes = []
    try:
        proc = popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                     stderr=subprocess.PIPE, env=env, close_fds=True, start_new_session=True,
                     cwd=working_directory)
        pipes = [("stdout", proc.stdout), ("stderr", proc.stderr)]
        for name, pipe in pipes:
            os.set_blocking(pipe.fileno(), False)
            selector.register(pipe, selectors.EVENT_READ, name)
        kill_deadline = None
        while selector.get_map() or proc.poll() is None:
            if disposition == "exited" and monotonic() - started >= timeout_seconds:
                disposition = "timeout"
                _kill_child_group(proc)
                kill_deadline = monotonic() + CHILD_KILL_DRAIN_SECONDS
            events = selector.select(0.05) if selector.get_map() else []
            if not selector.get_map() and proc.poll() is None:
                time.sleep(0.05)
            for key, _ in events:
                name = key.data
                try:
                    block = os.read(key.fd, 8192)
                except BlockingIOError:
                    continue
                except OSError as exc:
                    if disposition == "exited":
                        disposition = "read-error"
                        failure = f"{type(exc).__name__}: {exc}"
                        _kill_child_group(proc)
                        kill_deadline = monotonic() + CHILD_KILL_DRAIN_SECONDS
                    continue
                if not block:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                    continue
                remaining = output_limit - counts[name]
                if remaining > 0:
                    parts[name].append(block[:remaining])
                    counts[name] += min(len(block), remaining)
                if len(block) > remaining and disposition == "exited":
                    disposition = "output-limit"
                    _kill_child_group(proc)
                    kill_deadline = monotonic() + CHILD_KILL_DRAIN_SECONDS
            if kill_deadline is not None and monotonic() >= kill_deadline:
                for key in list(selector.get_map().values()):
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                break
        if proc.poll() is None:
            _kill_child_group(proc)
        code = proc.wait(timeout=CHILD_KILL_DRAIN_SECONDS)
    except BaseException as exc:
        failure = f"{type(exc).__name__}: {exc}"
        if proc is None:
            disposition = "launch-error"
        elif disposition == "exited":
            disposition = "runner-error"
        if proc is not None:
            _kill_child_group(proc)
            try:
                proc.kill()
            except OSError as kill_error:
                failure = f"{failure}; final kill failed: {type(kill_error).__name__}: {kill_error}"
            try:
                code = proc.wait(timeout=CHILD_KILL_DRAIN_SECONDS)
            except subprocess.TimeoutExpired as wait_error:
                code = None
                disposition = "cleanup-timeout"
                failure = (f"{failure}; child remained unreaped after bounded cleanup wait: "
                           f"{type(wait_error).__name__}: {wait_error}")
    finally:
        for key in list(selector.get_map().values()):
            selector.unregister(key.fileobj)
        selector.close()
        if proc is not None:
            for pipe in (proc.stdout, proc.stderr):
                if pipe is not None and not pipe.closed:
                    pipe.close()
    stdout = b"".join(stdout_parts)
    stderr = b"".join(stderr_parts)
    stdout_path = evidence_dir / f"{stage}.stdout.log"
    stderr_path = evidence_dir / f"{stage}.stderr.log"
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    ended_utc = datetime.now(timezone.utc).isoformat()
    command_receipt = {
        "stage": stage,
        "argv": argv,
        "cwd": working_directory,
        "started_at_utc": started_utc,
        "ended_at_utc": ended_utc,
        "duration_seconds": max(0.0, monotonic() - started),
        "exit_code": code,
        "disposition": disposition,
        "failure": failure,
        "stdout_path": stdout_path.name,
        "stdout_bytes": len(stdout),
        "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
        "stderr_path": stderr_path.name,
        "stderr_bytes": len(stderr),
        "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
        "output_limit_bytes": output_limit,
        "timeout_seconds": timeout_seconds,
    }
    _write_json(evidence_dir / f"{stage}.command.json", command_receipt)
    if disposition == "timeout":
        raise InstallError(f"{stage} child exceeded {timeout_seconds}s timeout")
    if disposition == "output-limit":
        raise InstallError(f"{stage} child exceeded captured output limit")
    if disposition == "read-error":
        raise InstallError(f"{stage} child output read failed: {failure}")
    if disposition == "launch-error":
        raise InstallError(f"{stage} child failed to launch: {failure}")
    if disposition == "runner-error":
        raise InstallError(f"{stage} child runner failed: {failure}")
    if disposition == "cleanup-timeout":
        raise InstallError(f"{stage} child cleanup timed out: {failure}")
    return code, stdout, stderr


def _kill_child_group(proc) -> None:
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    except OSError:
        try:
            proc.kill()
        except OSError:
            pass


def _version_json(executable: Path, args: list[str], stage: str, evidence: Path,
                  env: dict[str, str], *, timeout: int = CHILD_TIMEOUT_SECONDS,
                  popen=subprocess.Popen) -> dict:
    code, stdout, _ = _run_bounded_child(executable, args, stage, evidence, env,
                                        timeout_seconds=timeout, popen=popen)
    if code != 0:
        raise InstallError(f"{stage} command exited {code}")
    try:
        value = json.loads(stdout)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise InstallError(f"{stage} did not emit valid JSON") from exc
    if not isinstance(value, dict):
        raise InstallError(f"{stage} JSON is not an object")
    return value


def _checksum_line(data: bytes, filename: str, expected_sha: str) -> None:
    try:
        lines = data.decode("ascii").splitlines()
    except UnicodeDecodeError as exc:
        raise InstallError("checksum asset is not ASCII") from exc
    matches = []
    for line in lines:
        if not line:
            continue
        match = re.fullmatch(r"([0-9a-f]{64})  ([^\s]+)", line)
        if not match:
            raise InstallError("malformed checksum entry")
        if match.group(2) == filename:
            matches.append(match.group(1))
    if len(matches) != 1 or matches[0] != expected_sha:
        raise InstallError("release checksum entry is absent, duplicated or incorrect")


def _validate_linux_amd64() -> None:
    if not sys.platform.startswith("linux") or platform.machine().lower() not in ("x86_64", "amd64"):
        raise InstallError("installer supports only Linux x86_64")


def install(
    output: str | Path,
    evidence_dir: str | Path,
    cosign_path: str,
    cosign_sha256: str,
    *,
    pins: ReleasePins = SYFT_LINUX_AMD64,
    open_url=_default_open,
    monotonic=time.monotonic,
    network_read_timeout: int = READ_TIMEOUT_SECONDS,
    asset_deadline: int = ASSET_DEADLINE_SECONDS,
    child_timeout: int = CHILD_TIMEOUT_SECONDS,
    popen=subprocess.Popen,
    check_platform=True,
) -> Path:
    if check_platform:
        _validate_linux_amd64()
    output = _safe_absolute_path(output, "output path")
    evidence = _safe_absolute_path(evidence_dir, "evidence directory")
    _safe_existing_parent(output, "output path")
    _safe_existing_parent(evidence, "evidence directory")
    if output.exists() or output.is_symlink():
        raise InstallError("output path must be fresh")
    if evidence.exists() or evidence.is_symlink():
        raise InstallError("evidence directory must be fresh")
    if output == evidence or output.is_relative_to(evidence) or evidence.is_relative_to(output.parent):
        raise InstallError("output and evidence paths must be disjoint")
    cosign, cosign_stat = _cosign_identity(cosign_path, cosign_sha256)
    evidence.mkdir(mode=0o700)
    receipt_path = evidence / "receipt.json"
    receipt = {"status": "running", "syft_version": pins.version, "platform": "linux/amd64",
               "output_path": str(output), "cosign_path": str(cosign), "stages": []}
    _write_json(receipt_path, receipt)
    published = False
    try:
        with tempfile.TemporaryDirectory(prefix="syft-install-", dir=output.parent) as temp_name:
            temporary = Path(temp_name)
            env = _command_environment(temporary)
            check_path = temporary / pins.checksum_name
            bundle_path = temporary / pins.bundle_name
            archive_path = temporary / pins.archive_name
            checksum_info = _download(pins.checksum_url, check_path, pins.checksum_size,
                                      pins.checksum_sha256, open_url=open_url, monotonic=monotonic,
                                      read_timeout=network_read_timeout, deadline_seconds=asset_deadline)
            _record(receipt_path, receipt, {"stage": "download-checksums", **checksum_info})
            bundle_info = _download(pins.bundle_url, bundle_path, pins.bundle_size,
                                    pins.bundle_sha256, open_url=open_url, monotonic=monotonic,
                                    read_timeout=network_read_timeout, deadline_seconds=asset_deadline)
            _record(receipt_path, receipt, {"stage": "download-signature-bundle", **bundle_info})
            _verify_cosign_identity(cosign, cosign_sha256, cosign_stat)
            version = _version_json(cosign, ["version", "--json"], "cosign-version", evidence,
                                    env, timeout=child_timeout, popen=popen)
            _verify_cosign_identity(cosign, cosign_sha256, cosign_stat)
            if version.get("gitVersion") != "v3.1.3":
                raise InstallError("Cosign version differs from trusted setup pin")
            _record(receipt_path, receipt, {"stage": "cosign-version", "gitVersion": version["gitVersion"],
                                            "stdout_sha256": _hash_file(evidence / "cosign-version.stdout.log")})
            verify_args = [
                "verify-blob", "--bundle", str(bundle_path),
                "--certificate-identity", WORKFLOW_IDENTITY,
                "--certificate-oidc-issuer", OIDC_ISSUER,
                "--certificate-github-workflow-repository", "anchore/syft",
                "--certificate-github-workflow-sha", pins.git_commit,
                "--certificate-github-workflow-ref", WORKFLOW_REF,
                str(check_path),
            ]
            _verify_cosign_identity(cosign, cosign_sha256, cosign_stat)
            code, _, _ = _run_bounded_child(cosign, verify_args, "cosign-verify-blob", evidence,
                                           env, timeout_seconds=child_timeout, popen=popen)
            _verify_cosign_identity(cosign, cosign_sha256, cosign_stat)
            if code != 0:
                raise InstallError(f"Cosign signature verification exited {code}")
            _record(receipt_path, receipt, {"stage": "cosign-verify-blob", "exit_code": code,
                                            "stdout_sha256": _hash_file(evidence / "cosign-verify-blob.stdout.log"),
                                            "stderr_sha256": _hash_file(evidence / "cosign-verify-blob.stderr.log")})
            _checksum_line(check_path.read_bytes(), pins.archive_name, pins.archive_sha256)
            _record(receipt_path, receipt, {"stage": "release-checksum-entry", "filename": pins.archive_name,
                                            "sha256": pins.archive_sha256})
            archive_info = _download(pins.archive_url, archive_path, pins.archive_size,
                                     pins.archive_sha256, open_url=open_url, monotonic=monotonic,
                                     read_timeout=network_read_timeout, deadline_seconds=asset_deadline)
            _record(receipt_path, receipt, {"stage": "download-linux-archive", **archive_info})
            extracted = temporary / "extracted"
            syft_binary = _extract_archive(archive_path, extracted, pins)
            binary_sha = _hash_file(syft_binary)
            if binary_sha != pins.binary_sha256:
                raise InstallError("extracted Syft binary SHA-256 differs from release pin")
            _record(receipt_path, receipt, {"stage": "extract-and-hash-binary", "binary_sha256": binary_sha})
            syft_info = _version_json(syft_binary, ["version", "-o", "json"], "syft-version",
                                     evidence, env, timeout=child_timeout, popen=popen)
            if (syft_info.get("application") != "syft" or syft_info.get("version") != pins.version
                    or syft_info.get("gitCommit") != pins.git_commit
                    or syft_info.get("platform") != "linux/amd64"):
                raise InstallError("Syft version identity differs from authenticated release")
            if _hash_file(syft_binary) != pins.binary_sha256:
                raise InstallError("Syft binary changed during version check")
            _record(receipt_path, receipt, {"stage": "syft-version", "version": syft_info,
                                            "binary_sha256": pins.binary_sha256,
                                            "stdout_sha256": _hash_file(evidence / "syft-version.stdout.log")})
            os.link(syft_binary, output)
            published = True
            if _hash_file(output) != pins.binary_sha256:
                raise InstallError("published Syft binary differs from verified staging file")
            receipt["status"] = "verified"
            receipt["published_sha256"] = pins.binary_sha256
            _write_json(receipt_path, receipt)
        return output
    except BaseException as exc:
        receipt["status"] = "failed"
        receipt["failure"] = f"{type(exc).__name__}: {exc}"
        try:
            _write_json(receipt_path, receipt)
        except OSError:
            pass
        if published:
            output.unlink(missing_ok=True)
        raise


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, help="Absolute fresh path for the verified Syft executable")
    parser.add_argument("--evidence-dir", required=True, help="Absolute fresh path for bounded receipt and child logs")
    parser.add_argument("--cosign-path", required=True, help="Absolute Cosign executable path from trusted CI setup")
    parser.add_argument("--cosign-sha256", required=True, help="SHA-256 from the same trusted CI setup")
    args = parser.parse_args()
    try:
        installed = install(args.output, args.evidence_dir, args.cosign_path, args.cosign_sha256)
    except (InstallError, OSError, HTTPError, URLError, subprocess.SubprocessError) as exc:
        parser.exit(1, f"syft installer failed: {exc}\n")
    print(installed)


if __name__ == "__main__":
    main()

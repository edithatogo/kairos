#!/usr/bin/env python3
"""Acquire an exact successful Kairos package run; never select the latest run."""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import selectors
import signal
import stat
import subprocess
import sys
import tempfile
import time
import zipfile

REPOSITORY = "edithatogo/kairos"
WORKFLOW = ".github/workflows/package-dry-run.yml"
MAX_BYTES = 2 * 1024 * 1024 * 1024
MAX_MEMBERS = 10000
DOWNLOAD_TIMEOUT_SECONDS = 300
API_TIMEOUT_SECONDS = 60
API_OUTPUT_LIMIT = 16 * 1024 * 1024
API_COMMAND_PREFIX = ["gh", "api"]
MAX_SUBPROCESS_TIMEOUT_SECONDS = 3600

def validate_main_dispatch_run(run: dict, run_id: int) -> str:
    """Return the source SHA only for an exact successful same-repository main dispatch."""
    repository = run.get("repository")
    head_repository = run.get("head_repository")
    if type(run_id) is not int or run_id <= 0 or type(run.get("id")) is not int or run["id"] != run_id:
        raise ValueError("selected workflow run ID differs")
    if not isinstance(repository, dict) or not isinstance(head_repository, dict):
        raise ValueError("selected workflow run lacks repository identity")
    if repository.get("full_name") != REPOSITORY or head_repository.get("full_name") != REPOSITORY:
        raise ValueError("main dispatch must originate in the expected repository")
    repository_id = repository.get("id")
    head_repository_id = head_repository.get("id")
    if type(repository_id) is not int or repository_id <= 0 or type(head_repository_id) is not int or head_repository_id != repository_id:
        raise ValueError("main dispatch repository IDs differ")
    if run.get("path") != WORKFLOW or run.get("status") != "completed" or run.get("conclusion") != "success":
        raise ValueError("expected successful completed package workflow")
    if run.get("event") != "workflow_dispatch" or run.get("head_branch") != "main" or run.get("pull_requests") != []:
        raise ValueError("source run must be a same-repository manual dispatch on main without PR identity")
    source_commit = run.get("head_sha")
    if not isinstance(source_commit, str) or not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ValueError("main dispatch head must be a full lowercase SHA")
    return source_commit


def validate_main_ancestry(source_commit: str, branch: dict, comparison: dict | None) -> dict:
    """Validate branch and Compare API fields; bind the requested head through URL + branch readback."""
    if not isinstance(source_commit, str) or not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ValueError("expected full lowercase source SHA")
    if not isinstance(branch, dict) or branch.get("name") != "main":
        raise ValueError("expected exact main branch response")
    commit = branch.get("commit")
    observed_main_sha = commit.get("sha") if isinstance(commit, dict) else None
    if not isinstance(observed_main_sha, str) or not re.fullmatch(r"[0-9a-f]{40}", observed_main_sha):
        raise ValueError("main branch response lacks a full lowercase commit SHA")
    if source_commit == observed_main_sha:
        if comparison is not None:
            raise ValueError("identical source/main SHA must not require a compare response")
        return {
            "observed_branch": "main",
            "source_sha": source_commit,
            "observed_main_sha": observed_main_sha,
            "compare_url": None,
            "base_sha": source_commit,
            "merge_base_sha": source_commit,
            "status": "identical",
            "ahead_by": 0,
            "behind_by": 0,
        }
    expected_url = f"https://api.github.com/repos/{REPOSITORY}/compare/{source_commit}...{observed_main_sha}"
    if not isinstance(comparison, dict) or comparison.get("url") != expected_url:
        raise ValueError("compare response URL differs from exact source/main request")
    base_commit = comparison.get("base_commit")
    merge_base_commit = comparison.get("merge_base_commit")
    if not isinstance(base_commit, dict) or base_commit.get("sha") != source_commit:
        raise ValueError("compare base SHA differs from source")
    if not isinstance(merge_base_commit, dict) or merge_base_commit.get("sha") != source_commit:
        raise ValueError("source SHA is not the compare merge base")
    ahead_by = comparison.get("ahead_by")
    behind_by = comparison.get("behind_by")
    if comparison.get("status") != "ahead" or type(ahead_by) is not int or ahead_by <= 0 or type(behind_by) is not int or behind_by != 0:
        raise ValueError("main does not descend from source with valid compare counts")
    return {
        "observed_branch": "main",
        "source_sha": source_commit,
        "observed_main_sha": observed_main_sha,
        "compare_url": expected_url,
        "base_sha": base_commit["sha"],
        "merge_base_sha": merge_base_commit["sha"],
        "status": "ahead",
        "ahead_by": ahead_by,
        "behind_by": behind_by,
    }


def select_artifact(run: dict, inventory: dict, run_id: int, source_commit: str, head_commit: str | None = None, source_info: dict | None = None, *, require_main_dispatch: bool = False) -> dict:
    if require_main_dispatch:
        derived_source = validate_main_dispatch_run(run, run_id)
        if source_commit != derived_source or head_commit is not None or source_info is not None:
            raise ValueError("strict main selection must use the run-derived source SHA only")
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ValueError("expected full lowercase source SHA")
    if run.get("id") != run_id or run.get("head_sha") != (head_commit or source_commit):
        raise ValueError("run identity or source SHA differs")
    if run.get("repository", {}).get("full_name") != REPOSITORY or run.get("head_repository", {}).get("full_name") != REPOSITORY:
        raise ValueError("run must belong to the expected repository")
    if run.get("path") != WORKFLOW or run.get("status") != "completed" or run.get("conclusion") != "success":
        raise ValueError("expected successful completed package workflow")
    if run.get("event") == "pull_request":
        prs = run.get("pull_requests", [])
        if not head_commit or not re.fullmatch(r"[0-9a-f]{40}", head_commit) or len(prs) != 1:
            raise ValueError("PR acquisition requires an explicit head SHA and one PR identity")
        pr = prs[0]
        if pr.get("head", {}).get("sha") != head_commit or not source_info or source_info.get("sha") != source_commit:
            raise ValueError("PR head or build commit metadata differs")
        if [p.get("sha") for p in source_info.get("parents", [])] != [pr.get("base", {}).get("sha"), head_commit]:
            raise ValueError("build commit is not the exact base/head merge")
    elif source_commit != run.get("head_sha"):
        raise ValueError("non-PR build SHA differs from run head")
    rows = inventory.get("artifacts")
    if not isinstance(rows, list) or inventory.get("total_count") != len(rows):
        raise ValueError("incomplete artifact inventory")
    matches = [a for a in rows if a.get("name") == "kairos-actual-package-archives-" + source_commit]
    if len(matches) != 1:
        raise ValueError("expected exactly one matching retained bundle")
    item = matches[0]
    origin = item.get("workflow_run")
    if not isinstance(origin, dict) or origin.get("head_sha") != run["head_sha"]:
        raise ValueError("artifact origin head differs from selected run")
    expected_ids = {"id": run_id, "repository_id": run["repository"].get("id"),
                    "head_repository_id": run["head_repository"].get("id")}
    for key, expected in expected_ids.items():
        if type(expected) is not int or expected <= 0 or type(origin.get(key)) is not int or origin[key] != expected:
            raise ValueError("artifact origin run or repository differs")
    if require_main_dispatch and origin.get("head_branch") != "main":
        raise ValueError("artifact origin branch differs from main")
    if item.get("expired") is not False or not re.fullmatch(r"sha256:[0-9a-f]{64}", str(item.get("digest", ""))):
        raise ValueError("artifact expired or lacks SHA-256 identity")
    if type(item.get("id")) is not int or item["id"] <= 0:
        raise ValueError("invalid artifact identity")
    return item

DARWIN_GROUP_PROBE_SECONDS = 1
DARWIN_GROUP_PROBE_BYTES = 65536


def _darwin_group_snapshot(pid: int) -> tuple[int, bytes, bytes] | None:
    """Capture only a bounded diagnostic from the system ps, without group recursion."""
    process = None
    selector = selectors.DefaultSelector()
    buffers = [bytearray(), bytearray()]
    try:
        process = subprocess.Popen(
            ['/bin/ps', '-o', 'pid=,pgid=,stat=', '-g', str(pid)],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            bufsize=0,
        )
        deadline = time.monotonic() + DARWIN_GROUP_PROBE_SECONDS
        for stream, buffer in zip((process.stdout, process.stderr), buffers):
            os.set_blocking(stream.fileno(), False)
            selector.register(stream, selectors.EVENT_READ, buffer)
        while selector.get_map() or process.poll() is None:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return None
            for key, _ in selector.select(min(remaining, 0.05)):
                budget = DARWIN_GROUP_PROBE_BYTES - sum(map(len, buffers))
                chunk = os.read(key.fd, min(8192, budget + 1))
                if not chunk:
                    selector.unregister(key.fileobj)
                elif len(chunk) > budget:
                    return None
                else:
                    key.data.extend(chunk)
        return process.returncode, bytes(buffers[0]), bytes(buffers[1])
    except (OSError, ValueError):
        return None
    finally:
        selector.close()
        if process is not None:
            if process.poll() is None:
                try:
                    process.kill()  # Fixed system ps has no descendants; avoid recursive group probes.
                except ProcessLookupError:
                    pass
                except OSError:
                    pass
                try:
                    process.wait(timeout=1)  # Also reap a child that exited before kill().
                except (OSError, subprocess.TimeoutExpired):
                    pass
            for stream in (process.stdout, process.stderr):
                if stream is not None:
                    stream.close()


def _darwin_group_has_live_members(pid: int) -> bool | None:
    """Disambiguate Darwin EPERM for an exited group without accepting live groups."""
    snapshot = _darwin_group_snapshot(pid)
    if snapshot is None:
        return None
    code, stdout, stderr = snapshot
    if (code not in (0, 1) or stderr or len(stdout) > DARWIN_GROUP_PROBE_BYTES
            or (code == 1 and stdout.strip())):
        return None
    try:
        rows = stdout.decode('ascii').splitlines()
    except UnicodeDecodeError:
        return None
    for row in rows:
        fields = row.split()
        if (len(fields) != 3 or not fields[0].isdigit() or int(fields[0]) <= 0
                or not fields[1].isdigit() or int(fields[1]) != pid
                or re.fullmatch(r'[A-Za-z][A-Za-z+<>]*', fields[2]) is None):
            return None
        if not fields[2].startswith('Z'):
            return True
    # A zombie has already exited and cannot retain an open stdout pipe.
    return False


def _process_group_exists(pid: int) -> bool:
    try:
        os.killpg(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        if sys.platform == 'darwin' and _darwin_group_has_live_members(pid) is False:
            return False
        return True
    return True

def _signal_process_group(process: subprocess.Popen, signum: int) -> None:
    """Accept EPERM only after reaping and independently excluding live members."""
    process.poll()  # Reap an exited leader before signaling its former process group.
    try:
        os.killpg(process.pid, signum)
    except ProcessLookupError:
        pass
    except PermissionError:
        # macOS can report EPERM for a just-exited, not-yet-reaped group leader.
        # Only treat that race as gone after waitpid reaps the leader and a fresh
        # group probe independently excludes live members. Darwin can keep a
        # reaped zombie visible to ps while killpg continues reporting EPERM.
        if process.poll() is None or _process_group_exists(process.pid):
            raise


def _stop_process_group(process: subprocess.Popen) -> None:
    """Terminate, escalate, and reap a process group within bounded waits."""
    process.poll()  # Reap an already-exited leader before checking or signaling its group.
    if _process_group_exists(process.pid):
        _signal_process_group(process, signal.SIGTERM)
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            process.poll()
            if not _process_group_exists(process.pid):
                break
            time.sleep(0.05)
        if _process_group_exists(process.pid):
            _signal_process_group(process, signal.SIGKILL)
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                process.poll()
                if not _process_group_exists(process.pid):
                    break
                time.sleep(0.05)
    if process.poll() is None:
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            _signal_process_group(process, signal.SIGKILL)
            process.wait(timeout=2)
    if _process_group_exists(process.pid):
        raise RuntimeError("subprocess group did not stop")

def _run_bounded_process(argv: list[str], limit: int, timeout: float,
                         output_path: Path | None = None) -> tuple[str, bytes | None]:
    """Stream bounded subprocess output on POSIX with an absolute deadline."""
    if os.name != "posix":
        raise RuntimeError("bounded acquisition subprocesses require POSIX process groups and pipe selectors")
    if (type(limit) is not int or limit < 0 or limit > MAX_BYTES
            or isinstance(timeout, bool) or not isinstance(timeout, (int, float))
            or not math.isfinite(timeout) or timeout <= 0 or timeout > MAX_SUBPROCESS_TIMEOUT_SECONDS):
        raise ValueError("invalid subprocess output or time limit")
    created_output = False
    output = output_path.open("xb") if output_path is not None else None
    created_output = output is not None
    digest = hashlib.sha256()
    total = 0
    chunks = [] if output is None else None
    process = None
    started = time.monotonic()
    try:
        process = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                   start_new_session=True, bufsize=0)
        assert process.stdout is not None
        descriptor = process.stdout.fileno()
        os.set_blocking(descriptor, False)
        selector = selectors.DefaultSelector()
        try:
            selector.register(process.stdout, selectors.EVENT_READ)
            while selector.get_map():
                remaining_time = timeout - (time.monotonic() - started)
                if remaining_time <= 0:
                    raise TimeoutError("subprocess timed out")
                ready = selector.select(min(remaining_time, 0.1))
                if not ready:
                    continue
                for key, _ in ready:
                    block = os.read(key.fd, min(1024 * 1024, limit - total + 1))
                    if not block:
                        selector.unregister(key.fileobj)
                        continue
                    if len(block) > limit - total:
                        raise ValueError("subprocess output exceeds byte limit")
                    total += len(block)
                    digest.update(block)
                    if output is not None:
                        output.write(block)
                    else:
                        chunks.append(block)
        finally:
            selector.close()
        remaining_time = timeout - (time.monotonic() - started)
        if remaining_time <= 0:
            raise TimeoutError("subprocess timed out")
        try:
            code = process.wait(timeout=remaining_time)
        except subprocess.TimeoutExpired as exc:
            raise TimeoutError("subprocess timed out") from exc
        if code:
            raise subprocess.CalledProcessError(code, argv)
        if _process_group_exists(process.pid):
            _stop_process_group(process)
            raise ValueError("subprocess left a child process running")
        if output is not None:
            output.flush()
        return "sha256:" + digest.hexdigest(), (b"".join(chunks) if chunks is not None else None)
    except BaseException as error:
        cleanup_error = None
        try:
            if process is not None:
                _stop_process_group(process)
        except BaseException as caught_cleanup_error:
            cleanup_error = caught_cleanup_error
        finally:
            try:
                if output is not None:
                    output.close()
                    output = None
            finally:
                if created_output and output_path is not None:
                    output_path.unlink(missing_ok=True)
        if cleanup_error is not None:
            raise RuntimeError(f"subprocess failed ({error}); process cleanup failed ({cleanup_error})") from cleanup_error
        raise
    finally:
        if output is not None:
            output.close()
        if process is not None and process.stdout is not None:
            process.stdout.close()

def download_verified(argv: list[str], archive: Path, limit: int = MAX_BYTES) -> str:
    """Stream, hash, and retain only a complete download within time and byte limits."""
    result, _ = _run_bounded_process(argv, limit, DOWNLOAD_TIMEOUT_SECONDS, archive)
    return result


def archive_sha256(archive: Path) -> str:
    digest = hashlib.sha256()
    total = 0
    with archive.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            total += len(block)
            if total > MAX_BYTES:
                raise ValueError("download exceeds byte limit")
            digest.update(block)
    return "sha256:" + digest.hexdigest()


def extract_verified(archive: Path, output: Path, digest: str) -> None:
    if archive.stat().st_size > MAX_BYTES:
        raise ValueError("download exceeds byte limit")
    if archive_sha256(archive) != digest:
        raise ValueError("download differs from GitHub artifact digest")
    if output.exists():
        raise ValueError("extraction output must not exist")
    with zipfile.ZipFile(archive) as z:
        infos = z.infolist()
        if not infos or len(infos) > MAX_MEMBERS or sum(i.file_size for i in infos) > MAX_BYTES:
            raise ValueError("artifact exceeds extraction limits")
        names = set()
        for i in infos:
            raw = i.filename[:-1] if i.is_dir() else i.filename
            name = PurePosixPath(raw)
            mode = i.external_attr >> 16
            if not raw or "\\" in raw or ":" in raw or name.is_absolute() or ".." in name.parts or name.as_posix() != raw:
                raise ValueError("unsafe artifact member path")
            if raw in names or i.flag_bits & 1 or stat.S_ISLNK(mode) or (stat.S_IFMT(mode) not in (0, stat.S_IFREG, stat.S_IFDIR)):
                raise ValueError("duplicate, encrypted or special artifact member")
            names.add(raw)
        output.mkdir()
        try:
            for i in infos:
                target = output / i.filename
                if i.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with z.open(i) as src, target.open("xb") as dst:
                        shutil.copyfileobj(src, dst)
        except Exception:
            shutil.rmtree(output)
            raise

def api(path: str) -> dict:
    _, payload = _run_bounded_process([*API_COMMAND_PREFIX, path], API_OUTPUT_LIMIT, API_TIMEOUT_SECONDS)
    return json.loads(payload)

def validate_source_commit_readback(commit: dict, source_commit: str, producer_head: str) -> dict:
    """Validate a native GitHub commit readback and its source/head relationship."""
    if not isinstance(commit, dict) or commit.get("sha") != source_commit:
        raise ValueError("source commit readback identity differs")
    tree = commit.get("tree")
    tree_sha = tree.get("sha") if isinstance(tree, dict) else None
    if not isinstance(tree_sha, str) or not re.fullmatch(r"[0-9a-f]{40}", tree_sha):
        raise ValueError("source commit readback lacks a full tree SHA")
    parents = commit.get("parents")
    if not isinstance(parents, list) or not parents:
        raise ValueError("source commit readback lacks parent records")
    normalized_parents = []
    seen_parents = set()
    for parent in parents:
        if (not isinstance(parent, dict)
                or not isinstance(parent.get("sha"), str)
                or not re.fullmatch(r"[0-9a-f]{40}", parent["sha"])
                or parent.get("url") != f"https://api.github.com/repos/{REPOSITORY}/git/commits/{parent['sha']}"
                or parent.get("html_url") != f"https://github.com/{REPOSITORY}/commit/{parent['sha']}"
                or parent["sha"] == source_commit):
            raise ValueError("source commit readback has an invalid parent record")
        if parent["sha"] in seen_parents:
            raise ValueError("source commit readback has duplicate parents")
        seen_parents.add(parent["sha"])
        normalized_parents.append({key: parent[key] for key in ("sha", "url", "html_url")})
    if source_commit != producer_head and producer_head not in seen_parents:
        raise ValueError("PR producer head is not a source commit parent")
    verification = commit.get("verification")
    if not isinstance(verification, dict) or verification.get("verified") is not True or verification.get("reason") != "valid":
        raise ValueError("source commit signature is not verified")
    return {"sha": source_commit, "tree": {"sha": tree_sha}, "parents": normalized_parents, "verification": verification}


def _write_json(path: Path, value: dict) -> None:
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")


def retain_acquisition(root: Path, run: dict, item: dict, source_commit: str,
                       source_info: dict, original_receipt: dict,
                       main_ancestry: dict | None, branch_readback: dict | None,
                       compare_readback: dict | None, downloaded_digest: str,
                       expected_ecosystems: set[str]) -> None:
    """Create the complete fresh acquisition directory; caller owns rollback."""
    producer_head = run["head_sha"]
    source_identity = validate_source_commit_readback(source_info, source_commit, producer_head)
    bundle_path = root / "bundle"
    if not bundle_path.is_dir():
        raise ValueError("verified extracted bundle directory is missing")
    index = json.loads((bundle_path / "ARCHIVE-INDEX.json").read_text())
    if index.get("source_commit") != source_commit:
        raise ValueError("retained bundle has a different source SHA")
    artifacts = index.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts or any(not isinstance(row, dict) for row in artifacts):
        raise ValueError("retained bundle has no indexed archive rows")
    ecosystems = sorted({row.get("ecosystem") for row in artifacts})
    if ecosystems != sorted(expected_ecosystems):
        raise ValueError("retained bundle has invalid ecosystem rows")
    tree_sha = source_identity["tree"]["sha"]
    outer_receipt = {
        "exit_status": 0,
        "artifact_id": item["id"],
        "workflow_run": run["id"],
        "producer_checkout_source": source_commit,
        "producer_pr_head": producer_head,
        "producer_tree": tree_sha,
        "archive_zip_sha256": downloaded_digest.removeprefix("sha256:"),
        "actual_archive_count": len(artifacts),
        "ecosystems": ecosystems,
        "scope": "Actual retained archive structural/source acquisition verification; no producer SLSA attestation, release or registry acceptance",
    }
    if main_ancestry is not None:
        outer_receipt["selection_policy"] = "same-repository-main-workflow-dispatch"
        outer_receipt["main_ancestry"] = main_ancestry
    records = {
        "acquisition.json": original_receipt,
        "receipt.json": outer_receipt,
        "artifact-metadata.json": item,
        "source-commit-readback.json": {source_commit: source_identity},
        "source-commit-api-readback.json": {source_commit: source_info},
        "run-metadata.json": run,
    }
    if branch_readback is not None:
        records["branch-main-readback.json"] = branch_readback
    if compare_readback is not None:
        records["compare-main-readback.json"] = compare_readback
    for name, value in records.items():
        _write_json(root / name, value)

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--source-commit", help="Exact built commit (required for historical/PR selection)")
    parser.add_argument("--head-commit", help="Explicit PR head; source commit remains exact built merge")
    parser.add_argument("--require-main-dispatch", action="store_true",
                        help="Select only a successful same-repository manual package run on main")
    output_group = parser.add_mutually_exclusive_group(required=True)
    output_group.add_argument("--output", type=Path, help="Legacy extracted bundle directory")
    output_group.add_argument("--acquisition-output", type=Path,
                              help="Fresh directory retaining the bundle, original ZIP, and admission records")
    args = parser.parse_args()
    output = args.output if args.output is not None else args.acquisition_output
    output_exists = output.exists() or (args.acquisition_output is not None and output.is_symlink())
    if args.run_id <= 0 or output_exists:
        parser.error("positive run ID and nonexistent output required")
    if args.require_main_dispatch:
        if args.source_commit is not None or args.head_commit is not None:
            parser.error("main dispatch derives source SHA from the selected run; SHA overrides are forbidden")
    elif args.source_commit is None:
        parser.error("--source-commit is required unless --require-main-dispatch is selected")
    run = api(f"repos/{REPOSITORY}/actions/runs/{args.run_id}")
    main_ancestry = None
    branch_readback = None
    compare_readback = None
    if args.require_main_dispatch:
        source_commit = validate_main_dispatch_run(run, args.run_id)
        branch = api(f"repos/{REPOSITORY}/branches/main")
        branch_readback = branch
        if not isinstance(branch, dict) or branch.get("name") != "main":
            raise ValueError("expected exact main branch response")
        branch_commit = branch.get("commit")
        if not isinstance(branch_commit, dict):
            raise ValueError("main branch response lacks a commit object")
        comparison = None
        observed_main = branch_commit.get("sha")
        if not isinstance(observed_main, str) or not re.fullmatch(r"[0-9a-f]{40}", observed_main):
            raise ValueError("main branch response lacks a full lowercase commit SHA")
        if observed_main != source_commit:
            comparison = api(f"repos/{REPOSITORY}/compare/{source_commit}...{observed_main}")
            compare_readback = comparison
        main_ancestry = validate_main_ancestry(source_commit, branch, comparison)
        inventory = api(f"repos/{REPOSITORY}/actions/runs/{args.run_id}/artifacts?per_page=100")
        source_info = None
        item = select_artifact(run, inventory, args.run_id, source_commit,
                               require_main_dispatch=True)
    else:
        source_commit = args.source_commit
        inventory = api(f"repos/{REPOSITORY}/actions/runs/{args.run_id}/artifacts?per_page=100")
        source_info = api(f"repos/{REPOSITORY}/commits/{source_commit}") if run.get("event") == "pull_request" else None
        item = select_artifact(run, inventory, args.run_id, source_commit, args.head_commit, source_info)
    spec = importlib.util.spec_from_file_location("bundle", Path(__file__).with_name("build_package_archive_bundle.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    if args.acquisition_output is None:
        output.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=output.parent) as temp:
            archive = Path(temp) / "download.zip"
            downloaded_digest = download_verified(["gh", "api", f"repos/{REPOSITORY}/actions/artifacts/{item['id']}/zip"], archive)
            if downloaded_digest != item["digest"]:
                raise ValueError("download differs from GitHub artifact digest")
            tree = Path(temp) / "bundle"
            extract_verified(archive, tree, item["digest"])
            module.verify(tree)
            index = json.loads((tree / "ARCHIVE-INDEX.json").read_text())
            if index["source_commit"] != source_commit:
                raise ValueError("retained bundle has a different source SHA")
            receipt = output.with_name(output.name + ".acquisition.json")
            if receipt.exists():
                raise ValueError("acquisition receipt already exists")
            receipt_data = {"repository": REPOSITORY, "run_id": args.run_id,
                "source_commit": source_commit, "head_commit": run["head_sha"], "build_commit_parents": source_info.get("parents", []) if source_info else [], "artifact_id": item["id"],
                "artifact_digest": item["digest"], "archive_index_sha256": hashlib.sha256((tree / "ARCHIVE-INDEX.json").read_bytes()).hexdigest(),
                "scope": "verified acquisition; not original build provenance or release acceptance"}
            if args.require_main_dispatch:
                receipt_data["selection_policy"] = "same-repository-main-workflow-dispatch"
                receipt_data["main_ancestry"] = main_ancestry
            receipt.write_text(json.dumps(receipt_data, indent=2) + "\n")
            shutil.move(str(tree), output)
    else:
        source_info = api(f"repos/{REPOSITORY}/git/commits/{source_commit}")
        validate_source_commit_readback(source_info, source_commit, run["head_sha"])
        original_receipt = {"repository": REPOSITORY, "run_id": args.run_id,
            "source_commit": source_commit, "head_commit": run["head_sha"], "build_commit_parents": source_info.get("parents", []), "artifact_id": item["id"],
            "artifact_digest": item["digest"], "archive_index_sha256": None,
            "scope": "verified acquisition; not original build provenance or release acceptance"}
        if args.require_main_dispatch:
            original_receipt["selection_policy"] = "same-repository-main-workflow-dispatch"
            original_receipt["main_ancestry"] = main_ancestry
        output.parent.mkdir(parents=True, exist_ok=True)
        output.mkdir()
        try:
            archive = output / f"{item['id']}.zip"
            downloaded_digest = download_verified(["gh", "api", f"repos/{REPOSITORY}/actions/artifacts/{item['id']}/zip"], archive)
            if downloaded_digest != item["digest"]:
                raise ValueError("download differs from GitHub artifact digest")
            extract_verified(archive, output / "bundle", item["digest"])
            module.verify(output / "bundle")
            index_path = output / "bundle" / "ARCHIVE-INDEX.json"
            index = json.loads(index_path.read_text())
            if index.get("source_commit") != source_commit:
                raise ValueError("retained bundle has a different source SHA")
            original_receipt["archive_index_sha256"] = hashlib.sha256(index_path.read_bytes()).hexdigest()
            if args.require_main_dispatch:
                original_receipt["selection_policy"] = "same-repository-main-workflow-dispatch"
                original_receipt["main_ancestry"] = main_ancestry
            retain_acquisition(output, run, item, source_commit, source_info, original_receipt,
                               main_ancestry, branch_readback, compare_readback, downloaded_digest,
                               set(module.ARCHIVES))
        except BaseException:
            shutil.rmtree(output)
            raise
    print("verified exact-run package acquisition")
if __name__ == "__main__":
    main()

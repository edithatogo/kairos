#!/usr/bin/env python3
"""Acquire an exact successful Kairos package run; never select the latest run."""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import tempfile
import zipfile

REPOSITORY = "edithatogo/kairos"
WORKFLOW = ".github/workflows/package-dry-run.yml"
MAX_BYTES = 2 * 1024 * 1024 * 1024
MAX_MEMBERS = 10000

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

def download_verified(argv: list[str], archive: Path, limit: int = MAX_BYTES) -> str:
    """Bound bytes written, hash incrementally, and reap a failed producer."""
    if type(limit) is not int or limit < 0:
        raise ValueError("invalid download byte limit")
    stream = archive.open("xb")  # Never remove a destination we did not create.
    process = None
    digest = hashlib.sha256()
    total = 0
    try:
        with stream:
            process = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            with process.stdout:
                while True:
                    block = process.stdout.read(min(1024 * 1024, limit - total + 1))
                    if not block:
                        break
                    if len(block) > limit - total:
                        raise ValueError("download exceeds byte limit")
                    stream.write(block)
                    digest.update(block)
                    total += len(block)
            code = process.wait()
            if code:
                raise subprocess.CalledProcessError(code, argv)
        return "sha256:" + digest.hexdigest()
    except BaseException:
        if process is not None:
            if process.poll() is None:
                process.kill()
            process.wait()
        archive.unlink(missing_ok=True)
        raise


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
    return json.loads(subprocess.check_output(["gh", "api", path]))

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--source-commit", help="Exact built commit (required for historical/PR selection)")
    parser.add_argument("--head-commit", help="Explicit PR head; source commit remains exact built merge")
    parser.add_argument("--require-main-dispatch", action="store_true",
                        help="Select only a successful same-repository manual package run on main")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.run_id <= 0 or args.output.exists():
        parser.error("positive run ID and nonexistent output required")
    if args.require_main_dispatch:
        if args.source_commit is not None or args.head_commit is not None:
            parser.error("main dispatch derives source SHA from the selected run; SHA overrides are forbidden")
    elif args.source_commit is None:
        parser.error("--source-commit is required unless --require-main-dispatch is selected")
    run = api(f"repos/{REPOSITORY}/actions/runs/{args.run_id}")
    main_ancestry = None
    if args.require_main_dispatch:
        source_commit = validate_main_dispatch_run(run, args.run_id)
        branch = api(f"repos/{REPOSITORY}/branches/main")
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
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=args.output.parent) as temp:
        temp = Path(temp)
        archive = temp / "download.zip"
        downloaded_digest = download_verified(["gh", "api", f"repos/{REPOSITORY}/actions/artifacts/{item['id']}/zip"], archive)
        if downloaded_digest != item["digest"]:
            raise ValueError("download differs from GitHub artifact digest")
        tree = temp / "bundle"
        extract_verified(archive, tree, item["digest"])
        spec = importlib.util.spec_from_file_location("bundle", Path(__file__).with_name("build_package_archive_bundle.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.verify(tree)
        index = json.loads((tree / "ARCHIVE-INDEX.json").read_text())
        if index["source_commit"] != source_commit:
            raise ValueError("retained bundle has a different source SHA")
        # Evidence is a sibling: adding it inside the bundle would change its verified tree.
        receipt = args.output.with_name(args.output.name + ".acquisition.json")
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
        shutil.move(str(tree), args.output)
    print("verified exact-run package acquisition")
if __name__ == "__main__":
    main()

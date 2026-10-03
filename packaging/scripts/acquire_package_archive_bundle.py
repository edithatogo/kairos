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

def select_artifact(run: dict, inventory: dict, run_id: int, source_commit: str, head_commit: str | None = None, source_info: dict | None = None) -> dict:
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
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--head-commit", help="Explicit PR head; source commit remains exact built merge")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.run_id <= 0 or args.output.exists():
        parser.error("positive run ID and nonexistent output required")
    run = api(f"repos/{REPOSITORY}/actions/runs/{args.run_id}")
    inventory = api(f"repos/{REPOSITORY}/actions/runs/{args.run_id}/artifacts?per_page=100")
    source_info = api(f"repos/{REPOSITORY}/commits/{args.source_commit}") if run.get("event") == "pull_request" else None
    item = select_artifact(run, inventory, args.run_id, args.source_commit, args.head_commit, source_info)
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
        if index["source_commit"] != args.source_commit:
            raise ValueError("retained bundle has a different source SHA")
        # Evidence is a sibling: adding it inside the bundle would change its verified tree.
        receipt = args.output.with_name(args.output.name + ".acquisition.json")
        if receipt.exists():
            raise ValueError("acquisition receipt already exists")
        receipt.write_text(json.dumps({"repository": REPOSITORY, "run_id": args.run_id,
            "source_commit": args.source_commit, "head_commit": run["head_sha"], "build_commit_parents": source_info.get("parents", []) if source_info else [], "artifact_id": item["id"],
            "artifact_digest": item["digest"], "archive_index_sha256": hashlib.sha256((tree / "ARCHIVE-INDEX.json").read_bytes()).hexdigest(),
            "scope": "verified acquisition; not original build provenance or release acceptance"}, indent=2) + "\n")
        shutil.move(str(tree), args.output)
    print("verified exact-run package acquisition")
if __name__ == "__main__":
    main()

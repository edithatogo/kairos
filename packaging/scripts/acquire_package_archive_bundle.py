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

def select_artifact(run: dict, inventory: dict, run_id: int, source_commit: str) -> dict:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ValueError("expected full lowercase source SHA")
    if run.get("id") != run_id or run.get("head_sha") != source_commit:
        raise ValueError("run identity or source SHA differs")
    if run.get("repository", {}).get("full_name") != REPOSITORY or run.get("head_repository", {}).get("full_name") != REPOSITORY:
        raise ValueError("run must belong to the expected repository")
    if run.get("path") != WORKFLOW or run.get("status") != "completed" or run.get("conclusion") != "success":
        raise ValueError("expected successful completed package workflow")
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

def extract_verified(archive: Path, output: Path, digest: str) -> None:
    if archive.stat().st_size > MAX_BYTES:
        raise ValueError("download exceeds byte limit")
    if "sha256:" + hashlib.sha256(archive.read_bytes()).hexdigest() != digest:
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
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.run_id <= 0 or args.output.exists():
        parser.error("positive run ID and nonexistent output required")
    run = api(f"repos/{REPOSITORY}/actions/runs/{args.run_id}")
    inventory = api(f"repos/{REPOSITORY}/actions/runs/{args.run_id}/artifacts?per_page=100")
    item = select_artifact(run, inventory, args.run_id, args.source_commit)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=args.output.parent) as temp:
        temp = Path(temp)
        archive = temp / "download.zip"
        with archive.open("wb") as stream:
            subprocess.run(["gh", "api", f"repos/{REPOSITORY}/actions/artifacts/{item['id']}/zip"], stdout=stream, check=True)
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
            "source_commit": args.source_commit, "artifact_id": item["id"],
            "artifact_digest": item["digest"], "archive_index_sha256": hashlib.sha256((tree / "ARCHIVE-INDEX.json").read_bytes()).hexdigest(),
            "scope": "verified acquisition; not original build provenance or release acceptance"}, indent=2) + "\n")
        shutil.move(str(tree), args.output)
    print("verified exact-run package acquisition")
if __name__ == "__main__":
    main()

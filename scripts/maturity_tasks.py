#!/usr/bin/env python3
"""Check/show bounded Conductor tasks; never execute commands or grant authority."""
from __future__ import annotations

import argparse
import json
from pathlib import Path, PurePosixPath
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CATALOG = Path("conductor/execution/maturity-tasks.json")


def enabled_features(manifest: dict, argv: list[str]) -> set[str]:
    features = manifest.get("features", {})
    selected = set(features if "--all-features" in argv else [])
    if "--no-default-features" not in argv:
        selected.update(features.get("default", []))
    for index, value in enumerate(argv[:-1]):
        if value == "--features":
            selected.update(re.split(r"[,\s]+", argv[index + 1]))
    while True:
        expanded = selected | {
            item for feature in selected for item in features.get(feature, [])
            if item in features
        }
        if expanded == selected:
            return selected
        selected = expanded


def safe_path(value: object, root: Path, *, directory: bool = False) -> Path:
    if not isinstance(value, str) or not value or "\\" in value:
        raise ValueError(f"Invalid relative path: {value!r}")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or ":" in value:
        raise ValueError(f"Path escapes repository: {value!r}")
    if str(path) != value or (value == "." and not directory):
        raise ValueError(f"Path must be canonical: {value!r}")
    target = root / value
    if not target.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"Symlink escapes repository: {value!r}")
    return target


def validate(data: object, root: Path) -> list[dict]:
    if not isinstance(data, dict) or data.get("schema_version") != 1:
        raise ValueError("Expected schema_version 1")
    tasks = data.get("tasks")
    if not isinstance(tasks, list) or not tasks:
        raise ValueError("Expected nonempty task list")
    packages, manifests = {}, {}
    for manifest in (root / "crates").glob("*/Cargo.toml"):
        parsed = tomllib.loads(manifest.read_text())
        package = parsed["package"]["name"]
        packages[package] = manifest.parent
        manifests[package] = parsed
    by_id = {}
    for task in tasks:
        if not isinstance(task, dict):
            raise ValueError("Task must be an object")
        task_id = task.get("id", "")
        if not isinstance(task_id, str) or not re.fullmatch(r"[KS]\d{2}\.[a-z0-9]+", task_id):
            raise ValueError(f"Invalid task id: {task_id!r}")
        if task_id in by_id:
            raise ValueError(f"Duplicate task: {task_id}")
        by_id[task_id] = task
        for field in ("title", "parent", "acceptance", "stop"):
            if not isinstance(task.get(field), str) or not task[field].strip():
                raise ValueError(f"{task_id}: missing {field}")
        if task.get("kind") not in {"implementation", "regression", "design", "qualification"}:
            raise ValueError(f"{task_id}: invalid kind")
        if task.get("executor") not in {"local", "specialist"}:
            raise ValueError(f"{task_id}: invalid executor")
        if task["kind"] == "design" and task["executor"] != "specialist":
            raise ValueError(f"{task_id}: unresolved design needs a specialist")
        if "status" in task:
            raise ValueError(f"{task_id}: status belongs in the existing track plan")
        for field, limit in (("read", 8), ("write", 5)):
            values = task.get(field)
            if not isinstance(values, list) or not 1 <= len(values) <= limit:
                raise ValueError(f"{task_id}: {field} requires 1..{limit} files")
            if not all(isinstance(v, str) for v in values) or len(set(values)) != len(values):
                raise ValueError(f"{task_id}: invalid/duplicate {field} paths")
            for value in values:
                target = safe_path(value, root)
                if target.is_dir() or (field == "read" and not target.is_file()):
                    raise ValueError(f"{task_id}: {field} path is not a file: {value}")
        dependencies = task.get("requires")
        if not isinstance(dependencies, list) or not all(isinstance(d, str) for d in dependencies):
            raise ValueError(f"{task_id}: invalid dependencies")
        if len(set(dependencies)) != len(dependencies):
            raise ValueError(f"{task_id}: duplicate dependency")
        checks = task.get("checks")
        if not isinstance(checks, list) or not 1 <= len(checks) <= 3:
            raise ValueError(f"{task_id}: expected 1..3 check commands")
        for check in checks:
            if not isinstance(check, dict):
                raise ValueError(f"{task_id}: check must be an object")
            argv = check.get("argv")
            if not isinstance(argv, list) or not argv or not all(isinstance(a, str) and a for a in argv):
                raise ValueError(f"{task_id}: check must contain a nonempty argv")
            cwd = safe_path(check.get("cwd"), root, directory=True)
            if not cwd.is_dir():
                raise ValueError(f"{task_id}: check cwd does not exist")
            if argv[0] == "cargo" and "-p" in argv:
                index = argv.index("-p") + 1
                package = argv[index] if index < len(argv) else ""
                if package not in packages:
                    raise ValueError(f"{task_id}: unknown Cargo package {package}")
                enabled = enabled_features(manifests[package], argv)
                for flag, folder in (("--test", "tests"), ("--bench", "benches")):
                    if flag in argv:
                        index = argv.index(flag) + 1
                        name = argv[index] if index < len(argv) else ""
                        if not re.fullmatch(r"[A-Za-z0-9_-]+", name):
                            raise ValueError(f"{task_id}: invalid Cargo target")
                        target = packages[package] / folder / f"{name}.rs"
                        relative = target.relative_to(root).as_posix()
                        if not target.is_file() and relative not in task["write"]:
                            raise ValueError(f"{task_id}: missing target must be an explicit output: {relative}")
                        required = set()
                        for entry in manifests[package].get(flag[2:], []):
                            if entry.get("name") == name:
                                required.update(entry.get("required-features", []))
                        # Detect the simple crate-level gate used by existing targets.
                        # This is not a full Rust cfg evaluator or proof of nonzero tests.
                        if target.is_file():
                            gate = re.search(r'^#!\[cfg\(feature\s*=\s*"([^"]+)"\)\]', target.read_text(), re.M)
                            if gate:
                                required.add(gate[1])
                        if required - enabled:
                            raise ValueError(f"{task_id}: target requires features {sorted(required - enabled)}")
    visiting, complete = set(), set()

    def visit(task_id: str) -> None:
        if task_id not in by_id:
            raise ValueError(f"Unknown dependency: {task_id}")
        if task_id in visiting:
            raise ValueError(f"Dependency cycle: {task_id}")
        if task_id in complete:
            return
        visiting.add(task_id)
        for dependency in by_id[task_id]["requires"]:
            visit(dependency)
        visiting.remove(task_id)
        complete.add(task_id)

    for task_id in by_id:
        visit(task_id)
    expected = {f"S{i:02d}" for i in range(1, 18)}
    covered = {task.get("obligation") for task in tasks if task.get("obligation")}
    if covered != expected:
        raise ValueError(f"Scientific coverage mismatch: {sorted(expected ^ covered)}")
    return tasks


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("check", "list", "show"))
    parser.add_argument("task_id", nargs="?")
    args = parser.parse_args()
    try:
        tasks = validate(json.loads((ROOT / CATALOG).read_text()), ROOT)
        if args.command == "show":
            task = next((t for t in tasks if t["id"] == args.task_id), None)
            if task is None:
                raise ValueError(f"Unknown task: {args.task_id}")
            print(json.dumps(task, indent=2))
        elif args.command == "list":
            for task in tasks:
                print(f"{task['id']:18} {task['executor']:10} {task['title']}")
        else:
            print(f"PASS: {len(tasks)} bounded tasks; S01-S17 represented; structural validation only")
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Run the paired-transit fixture against the committed calibration crate."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / ".artifacts/mvp/C2.0.red-tests.paired-transit"
DISPOSABLE = ARTIFACTS / "disposable"
LOGS = ARTIFACTS / "logs"
FIXTURE = ROOT / "conformance/c20/transit_flow_c20.rs"
MISSING_MODULES = ("flow_bridge", "work_duration")
TEST_NAME = "actual_nonzero_route_consumes_stale_start_and_arrival_across_pause_resume_once"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", "-C", str(ROOT), *args], text=True).strip()


def run(argv: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)


def declared(source: str, name: str) -> bool:
    return bool(re.search(rf"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+{re.escape(name)}\s*;", source))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expect", choices=("red", "green"), default="red")
    args = parser.parse_args()
    if git("status", "--porcelain", "--untracked-files=all"):
        raise RuntimeError("runner requires a committed, clean workspace")
    commit = git("rev-parse", "HEAD")
    if DISPOSABLE.exists():
        shutil.rmtree(DISPOSABLE)
    if LOGS.exists():
        shutil.rmtree(LOGS)
    DISPOSABLE.mkdir(parents=True)
    LOGS.mkdir(parents=True)

    archive = subprocess.Popen(["git", "-C", str(ROOT), "archive", "--format=tar", commit], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    assert archive.stdout is not None
    with tarfile.open(fileobj=archive.stdout, mode="r|") as stream:
        stream.extractall(DISPOSABLE, filter="data")
    _, archive_error = archive.communicate()
    if archive.returncode:
        raise RuntimeError(f"git archive failed: {archive_error.decode(errors='replace')}")

    fixture_bytes = FIXTURE.read_bytes()
    crate = DISPOSABLE / "crates/kairo-ecs-calibration"
    src = crate / "src"
    (src / "paired_transit_c20.rs").write_bytes(fixture_bytes)
    lib_path = src / "lib.rs"
    lib_text = lib_path.read_text()
    manifest_path = crate / "Cargo.toml"
    manifest_before = manifest_path.read_bytes()
    lock_path = DISPOSABLE / "Cargo.lock"
    lock_before = lock_path.read_bytes()
    manifest = tomllib.loads(manifest_before.decode())
    files_present = {name: (src / f"{name}.rs").is_file() for name in MISSING_MODULES}
    declarations = {name: declared(lib_text, name) for name in MISSING_MODULES}
    flow_ready = (
        "flow" in manifest.get("features", {})
        and "kairo-ecs-des" in manifest.get("dependencies", {})
        and "kairo-ecs-abm" in manifest.get("dependencies", {})
    )
    ready = flow_ready and all(files_present.values()) and all(declarations.values())

    for name in MISSING_MODULES:
        if not files_present[name] and not declarations[name]:
            lib_text += f'\n#[cfg(test)]\n#[path = "{name}.rs"]\nmod {name};\n'
    lib_text += '\n#[cfg(test)]\n#[path = "paired_transit_c20.rs"]\nmod paired_transit_c20;\n'
    lib_path.write_text(lib_text)

    sysroot = run(["rustup", "run", "1.99.0", "rustc", "--print", "sysroot"])
    rustc = run(["rustup", "run", "1.99.0", "rustc", "--version"])
    cargo = run(["rustup", "run", "1.99.0", "cargo", "--version"])
    if any(result.returncode for result in (sysroot, rustc, cargo)):
        raise RuntimeError("Rust 1.99.0 toolchain unavailable")
    tool_bin = Path(sysroot.stdout.strip()) / "bin"
    cargo_path = Path(shutil.which("cargo") or "cargo").resolve()
    env = os.environ.copy()
    env["PATH"] = os.pathsep.join((str(tool_bin), str(cargo_path.parent), env.get("PATH", "")))
    env["RUSTC"] = str(tool_bin / "rustc")
    env["RUSTDOC"] = str(tool_bin / "rustdoc")
    env["CARGO_TARGET_DIR"] = str(DISPOSABLE / "target")
    env["CARGO_TERM_COLOR"] = "never"
    toolchain = f"rustc: {rustc.stdout.strip()}\ncargo: {cargo.stdout.strip()}\nRUSTC={env['RUSTC']}\nRUSTDOC={env['RUSTDOC']}\nCARGO_TARGET_DIR={env['CARGO_TARGET_DIR']}\n"
    (LOGS / "toolchain.log").write_text(toolchain)
    argv = ["cargo", "test", "--locked", "-p", "kairo-ecs-calibration", "--lib"]
    if ready:
        argv.extend(
            [
                "--features",
                "flow,kairo-ecs-abm/test-support",
                "--",
                "--exact",
                f"paired_transit_c20::{TEST_NAME}",
            ]
        )
    proc = run(argv, cwd=DISPOSABLE, env=env)
    raw = proc.stdout.encode()
    (LOGS / "cargo-test.log").write_bytes(raw)
    missing = sorted(
        name
        for name in MISSING_MODULES
        if re.search(rf"(?m)^error: couldn't find file `[^`]*{re.escape(name)}\.rs`$", proc.stdout)
    )
    expected_missing = [name for name in MISSING_MODULES if not files_present[name]]
    declaration_positions = {}
    for index, name in enumerate(MISSING_MODULES):
        match = re.search(rf"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+{re.escape(name)}\s*;", lib_text)
        declaration_positions[name] = match.start() if match else len(lib_text) + index
    first_expected_missing = min(
        expected_missing, key=lambda name: declaration_positions[name], default=None
    )
    compiler_errors = re.findall(r"(?m)^error(?:\[[^\]]+\])?: (?!could not compile)(.+)$", proc.stdout)
    only_first_missing_error = (
        first_expected_missing is not None
        and missing == [first_expected_missing]
        and len(compiler_errors) == 1
        and "couldn't find file" in compiler_errors[0]
    )
    manifest_lock_unchanged = manifest_before == manifest_path.read_bytes() and lock_before == lock_path.read_bytes()
    red = proc.returncode == 101 and only_first_missing_error and manifest_lock_unchanged
    green = bool(
        ready
        and proc.returncode == 0
        and re.search(rf"(?m)^test paired_transit_c20::{TEST_NAME} \.\.\. ok$", proc.stdout)
        and re.search(
            r"(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out;",
            proc.stdout,
        )
        and manifest_lock_unchanged
    )
    if args.expect == "red" and red:
        status = "expected_missing_production_module_red"
        oracle = "locked actual calibration crate exited 101 at the first exact absent production module path; other absent files are separately inventoried; no behavioral execution"
    elif args.expect == "green" and green:
        status = "named_paired_transit_fixture_passed"
        oracle = "exact named actual provider/Flow transit fixture passed once; other package tests were filtered out"
    else:
        status = "oracle_mismatch"
        oracle = (
            f"expected={args.expect}; cargo_exit={proc.returncode}; first_missing={first_expected_missing}; "
            f"diagnosed_missing={missing}; all_absent_files={expected_missing}; production_ready={ready}; "
            f"manifest_lock_unchanged={manifest_lock_unchanged}"
        )
    result = {
        "schema_version": 1,
        "task": "C2.0.red-tests.paired-transit",
        "status": status,
        "oracle": oracle,
        "expected": args.expect,
        "commit": commit,
        "fixture_sha256": sha256(fixture_bytes),
        "cargo_argv": argv,
        "cargo_cwd": str(DISPOSABLE),
        "cargo_exit_status": proc.returncode,
        "cargo_log": str(LOGS / "cargo-test.log"),
        "cargo_log_sha256": sha256(raw),
        "toolchain_log": str(LOGS / "toolchain.log"),
        "overlay": {
            "module_files_present": files_present,
            "module_declarations_present": declarations,
            "expected_missing_files": expected_missing,
            "first_expected_missing_file": first_expected_missing,
            "flow_feature_present": "flow" in manifest.get("features", {}),
            "des_dependency_present": "kairo-ecs-des" in manifest.get("dependencies", {}),
            "abm_dependency_present": "kairo-ecs-abm" in manifest.get("dependencies", {}),
            "source_lib_overlay_sha256": sha256(lib_text.encode()),
            "manifest_sha256_before_after": [sha256(manifest_before), sha256(manifest_path.read_bytes())],
            "lock_sha256_before_after": [sha256(lock_before), sha256(lock_path.read_bytes())],
        },
    }
    (ARTIFACTS / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    (LOGS / "commands.json").write_text(json.dumps({"archive_argv": ["git", "archive", "--format=tar", commit],
        "cargo_argv": argv, "cargo_cwd": str(DISPOSABLE), "commit": commit, "toolchain": toolchain,
        "cargo_exit_status": proc.returncode, "cargo_log_sha256": sha256(raw)}, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    return 0 if status != "oracle_mismatch" else 1


if __name__ == "__main__":
    raise SystemExit(main())

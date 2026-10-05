#!/usr/bin/env python3
"""Run the paired-flow fixture against a disposable committed calibration crate."""

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
ARTIFACTS = ROOT / ".artifacts/mvp/C2.0.red-tests.paired-flow"
DISPOSABLE = ARTIFACTS / "disposable"
LOGS = ARTIFACTS / "logs"
FIXTURE = ROOT / "conformance/c20/paired_flow_c20.rs"
TEST_MODULE = "macro_and_explicit_zero_micro_pair_actual_provider_work_without_transit_events_or_draws"
MISSING_MODULES = ("flow_bridge", "work_duration")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", "-C", str(ROOT), *args], text=True).strip()


def command(argv: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)


def module_declared(source: str, name: str) -> bool:
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

    archive = subprocess.Popen(
        ["git", "-C", str(ROOT), "archive", "--format=tar", commit],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert archive.stdout is not None
    with tarfile.open(fileobj=archive.stdout, mode="r|") as stream:
        stream.extractall(DISPOSABLE, filter="data")
    _, archive_error = archive.communicate()
    if archive.returncode:
        raise RuntimeError(f"git archive failed: {archive_error.decode(errors='replace')}")

    fixture_bytes = FIXTURE.read_bytes()
    fixture_copy = DISPOSABLE / "crates/kairo-ecs-calibration/src/paired_flow_c20.rs"
    fixture_copy.write_bytes(fixture_bytes)
    crate = DISPOSABLE / "crates/kairo-ecs-calibration"
    manifest_path = crate / "Cargo.toml"
    lib_path = crate / "src/lib.rs"
    manifest = tomllib.loads(manifest_path.read_text())
    lib_text = lib_path.read_text()
    files_present = {name: (crate / "src" / f"{name}.rs").is_file() for name in MISSING_MODULES}
    declarations = {name: module_declared(lib_text, name) for name in MISSING_MODULES}
    flow_feature = "flow" in manifest.get("features", {})
    flow_dependency = "kairo-ecs-des" in manifest.get("dependencies", {})
    ready = flow_feature and flow_dependency and all(files_present.values()) and all(declarations.values())

    if ready:
        lib_text += '\n#[cfg(test)]\n#[path = "paired_flow_c20.rs"]\nmod paired_flow_c20;\n'
    else:
        # Expose precise file-not-found diagnostics for the actual production
        # modules. These declarations add no implementation to the disposable copy.
        for name in MISSING_MODULES:
            if not files_present[name] and not declarations[name]:
                lib_text += f"\nmod {name};\n"
    lib_path.write_text(lib_text)

    sysroot = command(["rustup", "run", "1.99.0", "rustc", "--print", "sysroot"])
    rustc_version = command(["rustup", "run", "1.99.0", "rustc", "--version"])
    cargo_version = command(["rustup", "run", "1.99.0", "cargo", "--version"])
    if any(result.returncode for result in (sysroot, rustc_version, cargo_version)):
        raise RuntimeError("Rust 1.99.0 toolchain unavailable")
    tool_bin = Path(sysroot.stdout.strip()) / "bin"
    cargo_path = Path(shutil.which("cargo") or "cargo").resolve()
    env = os.environ.copy()
    env["PATH"] = os.pathsep.join((str(tool_bin), str(cargo_path.parent), env.get("PATH", "")))
    env["RUSTC"] = str(tool_bin / "rustc")
    env["RUSTDOC"] = str(tool_bin / "rustdoc")
    env["CARGO_TARGET_DIR"] = str(DISPOSABLE / "target")
    env["CARGO_TERM_COLOR"] = "never"
    toolchain = "\n".join(
        [
            f"rustc: {rustc_version.stdout.strip()}",
            f"cargo: {cargo_version.stdout.strip()}",
            f"RUSTC={env['RUSTC']}",
            f"RUSTDOC={env['RUSTDOC']}",
            f"CARGO_TARGET_DIR={env['CARGO_TARGET_DIR']}",
        ]
    )
    (LOGS / "toolchain.log").write_text(toolchain + "\n")

    argv = ["cargo", "test", "--locked", "-p", "kairo-ecs-calibration", "--lib"]
    if ready:
        argv.extend(["--features", "flow"])
    proc = command(argv, cwd=DISPOSABLE, env=env)
    raw_log = proc.stdout.encode()
    (LOGS / "cargo-test.log").write_bytes(raw_log)
    missing = sorted(
        name
        for name in MISSING_MODULES
        if re.search(rf"error\[E0583\]: file not found for module `{re.escape(name)}`", proc.stdout)
    )
    green = bool(
        ready
        and proc.returncode == 0
        and re.search(rf"(?m)^test paired_flow_c20::{re.escape(TEST_MODULE)} \.\.\. ok$", proc.stdout)
        and re.search(r"(?m)^test result: ok\. \d+ passed; 0 failed; 0 ignored;", proc.stdout)
    )
    red = proc.returncode == 101 and bool(missing)
    if args.expect == "red" and red:
        status = "expected_missing_production_module_red"
        oracle = "actual locked calibration test exited 101 with exact missing production module diagnostics"
    elif args.expect == "green" and green:
        status = "named_paired_fixture_passed"
        oracle = "named actual provider/Flow parity fixture passed; no test was ignored"
    else:
        status = "oracle_mismatch"
        oracle = (
            f"expected={args.expect}; cargo_exit={proc.returncode}; missing_modules={missing}; "
            f"production_modules_ready={ready}; named_fixture_passed={green}"
        )

    result = {
        "schema_version": 1,
        "task": "C2.0.red-tests.paired-flow",
        "status": status,
        "oracle": oracle,
        "expected": args.expect,
        "commit": commit,
        "fixture_sha256": sha256(fixture_bytes),
        "cargo_argv": argv,
        "cargo_cwd": str(DISPOSABLE),
        "cargo_exit_status": proc.returncode,
        "cargo_log": str(LOGS / "cargo-test.log"),
        "cargo_log_sha256": sha256(raw_log),
        "toolchain_log": str(LOGS / "toolchain.log"),
        "runner_overlay": {
            "fixture": str(fixture_copy.relative_to(DISPOSABLE)),
            "module_files_present": files_present,
            "module_declarations_present": declarations,
            "flow_feature_present": flow_feature,
            "des_dependency_present": flow_dependency,
            "source_lib_overlay_sha256": sha256(lib_text.encode()),
        },
    }
    (ARTIFACTS / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    (LOGS / "commands.json").write_text(
        json.dumps(
            {
                "archive_argv": ["git", "archive", "--format=tar", commit],
                "cargo_argv": argv,
                "cargo_cwd": str(DISPOSABLE),
                "commit": commit,
                "toolchain": toolchain,
                "cargo_exit_status": proc.returncode,
                "cargo_log_sha256": sha256(raw_log),
            },
            indent=2,
        )
        + "\n"
    )
    print(json.dumps(result, indent=2))
    return 0 if status in ("expected_missing_production_module_red", "named_paired_fixture_passed") else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001 - retain runner setup diagnostics for review
        LOGS.mkdir(parents=True, exist_ok=True)
        (LOGS / "runner-error.log").write_text(f"{type(exc).__name__}: {exc}\n")
        raise SystemExit(f"runner failed: {type(exc).__name__}: {exc}")

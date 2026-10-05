#!/usr/bin/env python3
"""Run the C2.0 route fixture in a disposable copy of the committed source."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile


ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / ".artifacts/mvp/C2.0.red-tests.routes"
DISPOSABLE = ARTIFACTS / "disposable"
LOGS = ARTIFACTS / "logs"
FIXTURE = ROOT / "conformance/c20/spatial_routes_c20.rs"
RUNNER = ROOT / "conformance/c20/run_routes_red.py"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", "-C", str(ROOT), *args], text=True).strip()


def run(argv: list[str], *, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=ROOT, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)


def main() -> int:
    if git("status", "--porcelain", "--untracked-files=all"):
        raise RuntimeError("runner requires a committed, clean workspace")
    base = git("rev-parse", "HEAD")
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    if DISPOSABLE.exists():
        shutil.rmtree(DISPOSABLE)
    if LOGS.exists():
        shutil.rmtree(LOGS)
    DISPOSABLE.mkdir(parents=True)
    LOGS.mkdir(parents=True)

    archive = subprocess.Popen(
        ["git", "-C", str(ROOT), "archive", "--format=tar", base],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    assert archive.stdout is not None
    with tarfile.open(fileobj=archive.stdout, mode="r|") as stream:
        stream.extractall(DISPOSABLE, filter="data")
    _, archive_error = archive.communicate()
    if archive.returncode:
        raise RuntimeError(f"git archive failed: {archive_error.decode(errors='replace')}")

    fixture_sha = digest(FIXTURE)
    overlay = DISPOSABLE / "conformance/c20/spatial_routes_c20.rs"
    overlay.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(FIXTURE, overlay)
    cargo_test = DISPOSABLE / "crates/kairo-ecs-abm/tests/spatial_routes_c20.rs"
    cargo_test.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(FIXTURE, cargo_test)

    sysroot_result = run(["rustup", "run", "1.99.0", "rustc", "--print", "sysroot"])
    toolchain_result = run(["rustup", "run", "1.99.0", "rustc", "--version"])
    cargo_result = run(["rustup", "run", "1.99.0", "cargo", "--version"])
    if any(result.returncode for result in (sysroot_result, toolchain_result, cargo_result)):
        raise RuntimeError("Rust 1.99.0 toolchain unavailable; see logs/toolchain.log")
    sysroot = Path(sysroot_result.stdout.strip())
    tool_bin = sysroot / "bin"
    cargo_bin = Path(shutil.which("cargo") or "cargo").resolve()
    env = os.environ.copy()
    env["PATH"] = os.pathsep.join((str(tool_bin), str(cargo_bin.parent), env.get("PATH", "")))
    env["RUSTC"] = str(tool_bin / "rustc")
    env["RUSTDOC"] = str(tool_bin / "rustdoc")
    env["CARGO_TARGET_DIR"] = str(DISPOSABLE / "target")
    env["CARGO_TERM_COLOR"] = "never"

    toolchain_text = "\n".join(
        [
            "rustup run 1.99.0 rustc --version",
            toolchain_result.stdout.strip(),
            "rustup run 1.99.0 cargo --version",
            cargo_result.stdout.strip(),
            f"RUSTC={env['RUSTC']}",
            f"RUSTDOC={env['RUSTDOC']}",
            f"CARGO_TARGET_DIR={env['CARGO_TARGET_DIR']}",
        ]
    )
    (LOGS / "toolchain.log").write_text(toolchain_text + "\n")

    argv = ["cargo", "test", "--locked", "-p", "kairo-ecs-abm", "--test", "spatial_routes_c20"]
    proc = subprocess.run(argv, cwd=DISPOSABLE, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    raw = proc.stdout
    (LOGS / "cargo-test.log").write_text(raw)
    joined = LOGS / "commands.log"
    joined.write_text(
        json.dumps(
            {
                "archive": ["git", "archive", "--format=tar", base],
                "overlay_fixture_sha256": fixture_sha,
                "cargo_argv": argv,
                "cargo_cwd": str(DISPOSABLE),
                "commit": base,
                "toolchain": toolchain_text,
                "cargo_exit_status": proc.returncode,
                "cargo_log_sha256": hashlib.sha256(raw.encode()).hexdigest(),
            },
            indent=2,
        )
        + "\n"
    )

    missing_module = proc.returncode == 101 and bool(
        re.search(r"error\[E0432\].*?unresolved import.*?kairo_ecs_abm::spatial", raw, re.S)
    )
    if proc.returncode == 0:
        status = "runtime_tests_passed"
        oracle = "native route conformance executed successfully; this is local fixture evidence only"
    elif missing_module:
        status = "expected_missing_spatial_api_red"
        oracle = "native Cargo test exited 101 with unresolved kairo_ecs_abm::spatial import"
    else:
        status = "unexpected_failure"
        oracle = "failure did not match the specifically expected missing spatial API red"

    result = {
        "schema_version": 1,
        "task": "C2.0.red-tests.routes",
        "status": status,
        "oracle": oracle,
        "commit": base,
        "fixture_sha256": fixture_sha,
        "cargo_argv": argv,
        "cargo_cwd": str(DISPOSABLE),
        "cargo_exit_status": proc.returncode,
        "cargo_log": str(LOGS / "cargo-test.log"),
        "cargo_log_sha256": hashlib.sha256(raw.encode()).hexdigest(),
        "toolchain_log": str(LOGS / "toolchain.log"),
        "commands_log": str(joined),
    }
    (ARTIFACTS / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    return 0 if status in ("expected_missing_spatial_api_red", "runtime_tests_passed") else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001 - preserve runner diagnostics as an auditable failure
        LOGS.mkdir(parents=True, exist_ok=True)
        (LOGS / "runner-error.log").write_text(f"{type(exc).__name__}: {exc}\n")
        print(f"runner failed: {type(exc).__name__}: {exc}", file=sys.stderr)
        raise SystemExit(1)

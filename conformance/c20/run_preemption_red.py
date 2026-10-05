#!/usr/bin/env python3
"""Run C2.0 paired-preemption tests against a disposable committed calibration crate."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / ".artifacts/packets/C2.0.red-tests.paired-preemption.json"
ARTIFACTS = ROOT / ".artifacts/mvp/C2.0.red-tests.paired-preemption"
FIXTURE = ROOT / "conformance/c20/preemption_flow_c20.rs"
MODULES = ("flow_bridge", "work_duration")
EXPECTED_TESTS = {
    "preemption_flow_c20::tests::suspend_preemption_resumes_remaining_work_and_preserves_completed_micro_transit",
    "preemption_flow_c20::tests::restart_preemption_rebuilds_original_template_without_resampling_or_transit_reset",
}

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def git(*args: str) -> str:
    return subprocess.check_output(["git", "-C", str(ROOT), *args], text=True).strip()

def command(argv: list[str], cwd: Path = ROOT, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, check=False)

def module_declared(source: str, name: str) -> bool:
    return bool(re.search(rf"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+{re.escape(name)}\s*;", source))

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expect", choices=("red", "green"), default="red")
    mode = parser.parse_args().expect
    packet = json.loads(PACKET.read_text())
    base = packet["base_commit"]
    for rel, expected in packet["input_hashes"].items():
        if sha256((ROOT / rel).read_bytes()) != expected:
            raise RuntimeError(f"frozen input hash changed: {rel}")
    commit = git("rev-parse", "HEAD")
    subprocess.run(["git", "-C", str(ROOT), "merge-base", "--is-ancestor", base, commit], check=True)
    if git("status", "--porcelain", "--untracked-files=all"):
        raise RuntimeError("runner requires a committed clean workspace")
    tree = git("rev-parse", "HEAD^{tree}")

    disp = ARTIFACTS / "disposable"
    logs = ARTIFACTS / "logs"
    if disp.exists():
        shutil.rmtree(disp)
    if logs.exists():
        shutil.rmtree(logs)
    disp.mkdir(parents=True)
    logs.mkdir(parents=True)
    archive = subprocess.run(["git", "-C", str(ROOT), "archive", "--format=tar", commit],
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if archive.returncode:
        raise RuntimeError(f"git archive failed: {archive.stderr.decode(errors='replace')}")
    with tarfile.open(fileobj=io.BytesIO(archive.stdout), mode="r:") as tar:
        tar.extractall(disp, filter="data")

    fixture = FIXTURE.read_bytes()
    crate = disp / "crates/kairo-ecs-calibration"
    lib = crate / "src/lib.rs"
    lib_text = lib.read_text()
    manifest_path = crate / "Cargo.toml"
    lock_path = disp / "Cargo.lock"
    manifest_hash = sha256(manifest_path.read_bytes())
    lock_hash = sha256(lock_path.read_bytes())
    files = {n: (crate / "src" / f"{n}.rs").is_file() for n in MODULES}
    declarations = {n: module_declared(lib_text, n) for n in MODULES}
    manifest = __import__("tomllib").loads(manifest_path.read_text())
    features = manifest.get("features", {})
    deps = manifest.get("dependencies", {})
    flow_ready = ("flow" in features and "kairo-ecs-des" in deps
                  and "kairo-ecs-abm" in deps)
    production_ready = flow_ready and all(files.values()) and all(declarations.values())
    (crate / "src/preemption_flow_c20.rs").write_bytes(fixture)
    lib_text += '\n#[cfg(test)]\n#[path = "preemption_flow_c20.rs"]\nmod preemption_flow_c20;\n'
    if not production_ready and mode == "red":
        missing_undeclared = [n for n in MODULES if not files[n] and not declarations[n]]
        if missing_undeclared:
            # One real absent production module gives an unambiguous parser red.
            lib_text += f"\nmod {missing_undeclared[0]};\n"
        elif not any(not present for present in files.values()):
            raise RuntimeError("production module declarations exist; expected red is no longer available")
    lib.write_text(lib_text)

    sysroot = command(["rustup", "run", "1.99.0", "rustc", "--print", "sysroot"])
    rustc_version = command(["rustup", "run", "1.99.0", "rustc", "--version"])
    cargo_version = command(["rustup", "run", "1.99.0", "cargo", "--version"])
    if any(x.returncode for x in (sysroot, rustc_version, cargo_version)):
        raise RuntimeError("Rust 1.99.0 toolchain unavailable")
    tool_bin = Path(sysroot.stdout.strip()) / "bin"
    env = os.environ.copy()
    cargo_path = Path(shutil.which("cargo") or "cargo").resolve()
    env["PATH"] = os.pathsep.join((str(tool_bin), str(cargo_path.parent), env.get("PATH", "")))
    env["RUSTC"], env["RUSTDOC"] = str(tool_bin / "rustc"), str(tool_bin / "rustdoc")
    env["CARGO_TARGET_DIR"] = str(disp / "target")
    env["CARGO_TERM_COLOR"] = "never"
    argv = ["cargo", "test", "--locked", "-p", "kairo-ecs-calibration", "--lib"]
    if production_ready:
        argv += ["--features", "flow"]
    proc = command(argv, disp, env)
    raw = proc.stdout.encode()
    (logs / "cargo-test.log").write_bytes(raw)
    (logs / "toolchain.log").write_text(
        f"rustc: {rustc_version.stdout.strip()}\ncargo: {cargo_version.stdout.strip()}\n"
        f"RUSTC={env['RUSTC']}\nRUSTDOC={env['RUSTDOC']}\nCARGO_TARGET_DIR={env['CARGO_TARGET_DIR']}\n")
    missing_errors = re.findall(r"(?m)^error\[E0583\]: file not found for module `([^`]+)`", proc.stdout)
    rust_errors = re.findall(r"(?m)^error(?:\[[^]]+\])?: .+$", proc.stdout)
    cargo_errors = [e for e in rust_errors if not e.startswith("error: could not compile ")]
    absent_in_archive = [n for n in MODULES if not (crate / "src" / f"{n}.rs").exists()]
    red = (not production_ready and proc.returncode == 101 and len(missing_errors) == 1
           and missing_errors[0] in absent_in_archive and cargo_errors == [
               f"error[E0583]: file not found for module `{missing_errors[0]}`"])
    observed = re.findall(r"(?m)^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)$", proc.stdout)
    counts = {}
    for name, status in observed:
        counts[name] = counts.get(name, 0) + 1
    summary = re.search(r"(?m)^test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;$", proc.stdout)
    green = (production_ready and proc.returncode == 0 and summary is not None
             and all(counts.get(n) == 1 and (n, "ok") in observed for n in EXPECTED_TESTS)
             and all(s == "ok" for _, s in observed))
    after_manifest_hash = sha256(manifest_path.read_bytes())
    after_lock_hash = sha256(lock_path.read_bytes())
    unchanged = manifest_hash == after_manifest_hash and lock_hash == after_lock_hash
    if not unchanged:
        raise RuntimeError("Cargo manifest or lockfile changed in disposable native run")
    verified = red if mode == "red" else green

    result = {
        "schema_version": 1, "task": packet["packet_id"],
        "status": "ready_for_review" if verified else "oracle_mismatch",
        "claim": "parser-only missing-production-module preparation; no runtime acceptance" if mode == "red" else "named actual Flow/provider preemption tests passed; no C2.0/C2.1 acceptance",
        "expected": mode, "packet_base": base, "commit": commit, "tree": tree,
        "fixture_sha256": sha256(fixture), "runner_sha256": sha256(Path(__file__).read_bytes()),
        "frozen_input_hashes_verified": True,
        "manifest_sha256_before_after": [manifest_hash, after_manifest_hash],
        "lock_sha256_before_after": [lock_hash, after_lock_hash],
        "production_ready": production_ready, "module_files_present": files,
        "module_declarations_present": declarations,
        "flow_feature_and_des_abm_dependencies_present": flow_ready,
        "cargo_argv": argv, "cargo_cwd": str(disp), "cargo_exit_status": proc.returncode,
        "missing_module_diagnostics": missing_errors if mode == "red" else [],
        "compiler_errors": rust_errors, "observed_tests": observed,
        "expected_tests": sorted(EXPECTED_TESTS), "summary": summary.group(0) if summary else None,
        "toolchain": {"rustc": rustc_version.stdout.strip(), "cargo": cargo_version.stdout.strip(),
                      "rustc_path": env["RUSTC"], "rustdoc_path": env["RUSTDOC"]},
        "cargo_log": str((logs / "cargo-test.log").relative_to(ROOT)),
        "cargo_log_sha256": sha256(raw),
        "limitations": ["Red is module-resolution preparation only, not runtime behavior.",
                        "A later green run must execute all lib tests and the two named fixtures without filtering or ignored tests.",
                        "C2.0/C2.1 runtime, transit, and checkpoint acceptance remain open."],
    }
    (ARTIFACTS / "result.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"status": result["status"], "commit": commit, "cargo_exit": proc.returncode,
                      "missing": missing_errors, "green": green,
                      "receipt": str((ARTIFACTS / "result.json").relative_to(ROOT))}, sort_keys=True))
    return 0 if verified else 1

if __name__ == "__main__":
    raise SystemExit(main())

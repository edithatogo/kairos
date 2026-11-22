#!/usr/bin/env python3
"""Build and exercise one native-core beta candidate on its actual host."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib
import venv


ROOT = Path(__file__).resolve().parents[2]
PROFILE = ROOT / "packaging/native-beta-scope.json"
RUST_CASE = ROOT / "tests/fixtures/minimal-consumer/src/main.rs"
C_CASE = ROOT / "tests/conformance/native-beta-abi.c"
PYTHON_CASE = ROOT / "bindings/python/native_tests/test_native_engine.py"
LICENSES = ("LICENSE-APACHE", "LICENSE-MIT")


def run(*args: str | Path, cwd: Path = ROOT, env: dict | None = None) -> None:
    subprocess.run(args, cwd=cwd, env=env, check=True)


def output(*args: str | Path, cwd: Path = ROOT, env: dict | None = None) -> str:
    return subprocess.check_output(args, cwd=cwd, env=env, text=True)


def require(condition: bool, detail: str) -> None:
    if not condition:
        raise SystemExit(detail)


def sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def read_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def receipt(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def archive(repo: Path, commit: str, package: Path, *paths: str) -> None:
    run("git", "-C", repo, "archive", "--format=tar.gz", "-o", package, commit, *paths)


def unpack(package: Path, directory: Path) -> None:
    directory.mkdir()
    with tarfile.open(package, "r:gz") as bundle:
        bundle.extractall(directory, filter="data")


def make_c_archive(
    cargo: tuple[str, ...], ffi_crate: str, work: Path, dist: Path, suffix: str,
) -> Path:
    link = ("--", "-Clink-arg=-Wl,-install_name,@rpath/libkairo_ecs_ffi.dylib") if suffix == ".dylib" else ()
    built = output(
        *cargo, "rustc", "--release", "--locked", "--no-default-features", "-p", ffi_crate,
        "--lib", "--crate-type", "cdylib", "--message-format=json-render-diagnostics", *link,
    )
    libraries = [
        Path(name) for line in built.splitlines()
        if (item := json.loads(line)).get("reason") == "compiler-artifact"
        and item["target"]["name"] == "kairo_ecs_ffi"
        for name in item["filenames"] if name.endswith(suffix)
    ]
    require(len(libraries) == 1, "Expected one native ABI library from Cargo")
    c_stage = work / "abi-build"
    (c_stage / "lib").mkdir(parents=True)
    (c_stage / "include").mkdir()
    shutil.copyfile(libraries[0], c_stage / "lib" / libraries[0].name)
    shutil.copyfile(ROOT / "include/kairo_ecs.h", c_stage / "include/kairo_ecs.h")
    for name in LICENSES:
        shutil.copyfile(ROOT / name, c_stage / name)
    path = dist / "kairos-c-abi.tar.gz"
    with tarfile.open(path, "w:gz") as bundle:
        for member in sorted(c_stage.iterdir()):
            bundle.add(member, arcname=member.name)
    return path


def run_installed_c(abi: Path, library: Path, clean: Path, env: dict) -> None:
    binary = clean / "c-case"
    linker = ["-ldl"] if sys.platform == "linux" else []
    run(
        "cc", "-std=c11", "-Wall", "-Wextra", "-Werror", "-I", abi / "include",
        C_CASE, "-L", library.parent, "-lkairo_ecs_ffi",
        f"-Wl,-rpath,{library.parent}", *linker, "-o", binary,
        cwd=clean, env=env,
    )
    run(binary, library, cwd=clean, env=env)


def run_installed_python(wheel: Path, library: Path, clean: Path, env: dict) -> None:
    environment = clean / "python"
    venv.EnvBuilder(with_pip=True).create(environment)
    python = environment / "bin/python"
    run(
        python, "-Im", "pip", "install", "--no-index", "--no-deps",
        "--disable-pip-version-check", wheel, cwd=clean, env=env,
    )
    admission = """
import importlib.metadata
from pathlib import Path
import sys
import kairo_ecs

assert Path(kairo_ecs.__file__).resolve().is_relative_to(Path(sys.prefix).resolve())
assert importlib.metadata.version("kairo-ecs") == sys.argv[1]
try:
    kairo_ecs.NativeEngine()
except kairo_ecs.NativeBackendUnavailable:
    pass
else:
    raise AssertionError("NativeEngine started without an installed library selection")
with kairo_ecs.NativeEngine(sys.argv[2]) as engine:
    assert Path(engine.library_path).resolve() == Path(sys.argv[2]).resolve()
    engine.schedule_at(11)
    assert engine.step() and engine.current_time_ticks == 11
"""
    run(python, "-Ic", admission, wheel.name.split("-")[1], library, cwd=clean, env=env)
    test_suite = clean / "native-tests"
    test_suite.mkdir()
    shutil.copyfile(PYTHON_CASE, test_suite / PYTHON_CASE.name)
    run(
        python, "-Im", "unittest", "discover", "-s", test_suite, "-v",
        cwd=clean, env={**env, "KAIRO_ECS_FFI_LIBRARY": str(library)},
    )


def external_dependencies(lock: Path) -> set[tuple]:
    return {
        (package["name"], package["version"], package["source"], package.get("checksum"))
        for package in read_toml(lock)["package"]
        if "source" in package
    }


def resolve(
    cargo: tuple[str, ...], consumer: Path, sdk: Path, allowed_crates: set[str],
    extra_lock: Path | None = None,
) -> dict:
    sdk_lock = sdk / "Cargo.lock"
    lock = consumer / "Cargo.lock"
    shutil.copyfile(sdk_lock, lock)
    metadata = json.loads(output(*cargo, "metadata", "--format-version", "1", cwd=consumer))
    frozen = external_dependencies(sdk_lock)
    if extra_lock is not None:
        # CareOps may add libraries absent from the SDK lock. Shared libraries
        # retain the locked provider versions.
        sdk_names = {entry[0] for entry in frozen}
        frozen.update(entry for entry in external_dependencies(extra_lock) if entry[0] not in sdk_names)
    drift = external_dependencies(lock) - frozen
    require(not drift, f"Dependencies outside the frozen locks: {sorted(drift)}")
    workspace_members = set(metadata["workspace_members"])
    packages = [
        {
            "name": package["name"],
            "version": package["version"],
            "manifest": str(Path(package["manifest_path"]).resolve().relative_to(sdk)),
        }
        for package in metadata["packages"]
        if package["id"] not in workspace_members and package["name"].startswith("kairo-ecs-")
    ]
    require(bool(packages), "The consumer did not resolve any installed SDK crate")
    active = output(*cargo, "tree", "--locked", "--offline", "--edges", "normal,dev", "--prefix", "none", cwd=consumer)
    active_crates = set(re.findall(r"^(kairo-ecs-[a-z-]+) v", active, re.MULTILINE))
    active_crates.difference_update(
        package["name"] for package in metadata["packages"] if package["id"] in workspace_members
    )
    require(bool(active_crates), "The consumer has no active SDK crates")
    require(active_crates <= allowed_crates, f"Unexpected active SDK crates: {active_crates - allowed_crates}")
    return {"packages": packages, "active_sdk_crates": sorted(active_crates), "lock_sha256": sha256(lock)}


def run_installed_rust(cargo: tuple[str, ...], clean: Path, sdk: Path, cfg: dict) -> dict:
    consumer = clean / "rust"
    (consumer / "src").mkdir(parents=True)
    shutil.copyfile(RUST_CASE, consumer / "src/main.rs")
    manifest = RUST_CASE.parents[1] / "Cargo.toml"
    template = manifest.read_text(encoding="utf-8").split("[dependencies]", 1)[0]
    dependencies = [
        f'{name} = {{ path = {json.dumps(str(sdk / "crates" / name))} }}'
        for name in read_toml(manifest)["dependencies"]
    ]
    patch = (sdk / "Cargo.toml").read_text(encoding="utf-8").split("[patch.crates-io]", 1)[1]
    (consumer / "Cargo.toml").write_text(
        template + "[dependencies]\n" + "\n".join(dependencies) + "\n\n[patch.crates-io]" + patch,
        encoding="utf-8",
    )
    sources = resolve(cargo, consumer, sdk, set(cfg["rust_crates"]))
    run(*cargo, "run", "--locked", "--offline", cwd=consumer)
    run_case(
        cargo, sdk, "-p", "kairo-ecs-abm", "--lib",
        case="tests::behavior_simulation_replays_entity_rng",
    )
    return sources


def run_case(cargo: tuple[str, ...], cwd: Path, *selection: str, case: str) -> None:
    result = output(*cargo, "test", "--locked", *selection, "--", case, "--exact", cwd=cwd)
    print(result, end="")
    require("running 1 test" in result and "1 passed; 0 failed" in result, f"Case did not pass: {case}")


def run_installed_careops(
    cargo: tuple[str, ...], clean: Path, sdk: Path, cfg: dict,
) -> dict:
    careops = cfg["careops"]
    repository = clean / "careops.git"
    run("git", "init", "-q", "--bare", repository)
    run("git", "-C", repository, "fetch", "--quiet", "--depth=1", careops["repository"], careops["revision"])
    require(
        output("git", "-C", repository, "rev-parse", "FETCH_HEAD").strip() == careops["revision"],
        "CareOps source revision mismatch",
    )
    package = careops["package"]
    source_archive = clean / "careops-source.tar.gz"
    archive(repository, careops["revision"], source_archive, "Cargo.toml", "Cargo.lock", f"crates/{package}")
    source = clean / "careops"
    unpack(source_archive, source)
    old_lock = source / "CareOps.lock"
    (source / "Cargo.lock").rename(old_lock)
    manifest = source / "Cargo.toml"
    required = {"workspace", "workspace.package", "workspace.dependencies", "patch.crates-io"}
    sections = re.split(r"(?=^\[)", manifest.read_text(encoding="utf-8"), flags=re.MULTILINE)
    selected = {section.split("]", 1)[0][1:]: section for section in sections if section.startswith("[")}
    require(required <= selected.keys(), "CareOps workspace sections changed")
    selected["workspace"] = re.sub(
        r"^members = .*", f'members = ["crates/{package}"]', selected["workspace"], flags=re.MULTILINE,
    )
    manifest.write_text("".join(selected[name] for name in selected if name in required), encoding="utf-8")
    (source / "libs").mkdir()
    (source / "libs/kairos").symlink_to(sdk, target_is_directory=True)
    sources = resolve(cargo, source, sdk, set(cfg["rust_crates"]), old_lock)
    run_case(cargo, source, "--offline", "-p", package, "--test", careops["test"], case=careops["case"])
    return {
        "revision": careops["revision"],
        "case": careops["case"],
        "case_sha256": sha256(source / "crates" / package / "tests" / f'{careops["test"]}.rs'),
        "source_archive_sha256": sha256(source_archive),
        **sources,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    output_root = parser.parse_args().output.resolve()
    cfg = json.loads(PROFILE.read_text(encoding="utf-8"))
    require(cfg["c_abi"] == 1, "The installed C and Python cases qualify ABI v1")
    require(platform.python_version() == cfg["python"], "Qualification requires the profiled CPython version")
    require(sys.implementation.name == "cpython", "Qualification requires CPython")
    rust = ("rustup", "run", cfg["rust"], "rustc")
    cargo = ("rustup", "run", cfg["rust"], "cargo")
    rustc = output(*rust, "-Vv")
    target = next(line.removeprefix("host: ") for line in rustc.splitlines() if line.startswith("host: "))
    require(target in cfg["platforms"], f"Unsupported qualification host {target}")
    require(
        (target == "aarch64-apple-darwin" and platform.machine() == "arm64")
        or (target == "x86_64-unknown-linux-gnu" and platform.machine() == "x86_64"),
        "Python and Rust must run on the same native architecture",
    )
    require(not output("git", "status", "--porcelain").strip(), "Qualification requires clean committed source")
    head = output("git", "rev-parse", "HEAD").strip()
    output_root.mkdir()
    work = output_root / "work"
    dist = output_root / "artifacts"
    clean = output_root / "installed"
    for directory in (work, dist, clean):
        directory.mkdir()
    os.environ["CARGO_TARGET_DIR"] = str(work / "cargo-target")
    os.environ["CARGO_BUILD_JOBS"] = "2"
    os.environ["CARGO_PROFILE_DEV_DEBUG"] = "0"
    os.environ["CARGO_INCREMENTAL"] = "0"

    sdk_package = dist / "kairos-rust-sdk.tar.gz"
    archive(ROOT, head, sdk_package, "Cargo.toml", "Cargo.lock", "crates", *LICENSES)
    c_package = make_c_archive(cargo, cfg["ffi_crate"], work, dist, ".dylib" if sys.platform == "darwin" else ".so")
    python_source = work / "python-source"
    python_archive = work / "python-source.tar.gz"
    archive(ROOT, head, python_archive, "bindings/python")
    unpack(python_archive, python_source)
    run(sys.executable, "-Im", "build", "--wheel", "--no-isolation", "--outdir", dist, python_source / "bindings/python")
    wheels = list(dist.glob("kairo_ecs-*-py3-none-any.whl"))
    require(len(wheels) == 1, "Expected one Python adapter wheel")
    wheel = wheels[0]
    digests = {artifact.name: sha256(artifact) for artifact in (sdk_package, c_package, wheel)}
    (dist / "SHA256SUMS").write_text(
        "".join(f"{digest}  {name}\n" for name, digest in sorted(digests.items())), encoding="utf-8",
    )
    for name, digest in digests.items():
        require(sha256(dist / name) == digest, f"Artifact checksum mismatch: {name}")

    sdk = clean / "sdk"
    abi = clean / "abi"
    unpack(sdk_package, sdk)
    unpack(c_package, abi)
    (library,) = (abi / "lib").iterdir()
    env = {
        key: value for key, value in os.environ.items()
        if key not in {"PYTHONPATH", "PYTHONHOME", "KAIRO_ECS_FFI_LIBRARY", "LD_LIBRARY_PATH"}
        and not key.startswith("DYLD_")
    }
    run_installed_c(abi, library, clean, env)
    run_installed_python(wheel, library, clean, env)
    rust_resolution = run_installed_rust(cargo, clean, sdk, cfg)
    careops = run_installed_careops(cargo, clean, sdk, cfg)
    receipt(dist / "installed-native-receipt.json", {
        "profile": cfg["profile"],
        "status": "pass",
        "source_commit": head,
        "target": target,
        "platform": platform.platform(),
        "rustc": rustc.splitlines(),
        "python": platform.python_version(),
        "c_abi": cfg["c_abi"],
        "artifacts_sha256": digests,
        "library_sha256": sha256(library),
        "rust_fixture_sha256": sha256(RUST_CASE),
        "c_fixture_sha256": sha256(C_CASE),
        "python_suite_sha256": sha256(PYTHON_CASE),
        "rust_installed": rust_resolution,
        "careops_installed": careops,
    })


if __name__ == "__main__":
    main()

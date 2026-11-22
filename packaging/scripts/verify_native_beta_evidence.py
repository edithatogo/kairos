"""Prepare SBOM inputs from one retained and reviewed native beta run."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import zipfile
from pathlib import Path

from qualify_native_beta import PROFILE, ROOT, receipt, require, sha256, unpack

CANDIDATE = ROOT / "conductor/tracks/113-beta-and-stable-release/metadata.json"
INSTALLATION = "installed-native-receipt.json"
SDK = "kairos-rust-sdk.tar.gz"
ABI = "kairos-c-abi.tar.gz"
WHEEL_NAME = re.compile(r"kairo_ecs-[a-z0-9][a-z0-9_.+!]*-py3-none-any\.whl")


def verify_run(run: dict, candidate: dict, repository: str) -> None:
    expected = {
        "id": candidate["installed_candidate_run_id"],
        "head_sha": candidate["installed_candidate_commit"],
        "head_branch": "main",
        "path": ".github/workflows/native-beta.yml",
        "event": "push",
        "status": "completed",
        "conclusion": "success",
    }
    for field, value in expected.items():
        require(run.get(field) == value, f"Installed beta run differs at {field}")
    for field in ("repository", "head_repository"):
        require(
            (run.get(field) or {}).get("full_name") == repository,
            f"Installed beta run has an unexpected {field}",
        )


def verify_artifacts(directory: Path, target: str, profile: dict, candidate: dict) -> dict:
    require(directory.is_dir() and not directory.is_symlink(), f"Missing bundle for {target}")
    files = {path.name: path for path in directory.iterdir()}
    wheels = [name for name in files if WHEEL_NAME.fullmatch(name)]
    require(len(wheels) == 1, f"Expected one Python wheel for {target}")
    distributions = {SDK, ABI, wheels[0]}
    require(
        files.keys() == distributions | {INSTALLATION, "SHA256SUMS"},
        f"Unexpected bundle files for {target}",
    )
    require(
        all(path.is_file() and not path.is_symlink() for path in files.values()),
        f"Non-regular bundle file for {target}",
    )
    expected_receipt = candidate["installed_candidate_receipt_sha256"][target]
    require(sha256(files[INSTALLATION]) == expected_receipt, f"Installation receipt digest differs for {target}")

    installed = json.loads(files[INSTALLATION].read_text(encoding="utf-8"))
    expected = {
        "status": "pass",
        "source_commit": candidate["installed_candidate_commit"],
        "profile": profile["profile"],
        "target": target,
        "python": profile["python"],
        "c_abi": profile["c_abi"],
    }
    for field, value in expected.items():
        require(installed.get(field) == value, f"Installation receipt differs at {field} for {target}")
    require(f"release: {profile['rust']}" in installed["rustc"], f"Rust version differs for {target}")
    require(f"host: {target}" in installed["rustc"], f"Rust target differs for {target}")
    careops = installed["careops_installed"]
    require(
        all(careops.get(field) == profile["careops"][field] for field in ("revision", "case")),
        f"Installed CareOps case differs for {target}",
    )
    digests = {name: sha256(files[name]) for name in sorted(distributions)}
    require(digests == installed["artifacts_sha256"], f"Tested distribution digest differs for {target}")
    sums = "".join(f"{digest}  {name}\n" for name, digest in digests.items())
    require(files["SHA256SUMS"].read_bytes() == sums.encode("ascii"), f"SHA256SUMS differs for {target}")
    return digests


def prepare(artifacts: Path, output: Path, run: dict, profile: dict, candidate: dict) -> None:
    targets = profile["platforms"]
    bundles = {target: artifacts / f"native-beta-{target}" for target in targets}
    require(
        {path.name for path in bundles.values()} == {path.name for path in artifacts.iterdir()},
        "Native beta bundle set differs from the profile",
    )
    require(
        set(targets) == candidate["installed_candidate_receipt_sha256"].keys(),
        "Candidate receipt targets differ from the profile",
    )
    digests = {
        target: verify_artifacts(path, target, profile, candidate) for target, path in bundles.items()
    }
    require(
        len({values[SDK] for values in digests.values()}) == 1,
        "The installed SDK differs between targets",
    )

    output.mkdir()
    evidence = output / "evidence"
    evidence.mkdir()
    receipt(
        evidence / "installation-run.json",
        {field: run[field] for field in ("id", "head_sha", "path", "html_url")},
    )
    subjects = []
    for target, bundle in bundles.items():
        retained = evidence / bundle.name
        retained.mkdir()
        for name in (INSTALLATION, "SHA256SUMS"):
            shutil.copyfile(bundle / name, retained / name)
        subjects.extend(f"{digest}  {bundle.name}/{name}\n" for name, digest in digests[target].items())
        components = output / "components" / target
        components.mkdir(parents=True)
        unpack(bundle / SDK, components / "sdk")
        unpack(bundle / ABI, components / "abi")
        wheel = next(name for name in digests[target] if WHEEL_NAME.fullmatch(name))
        with zipfile.ZipFile(bundle / wheel) as archive:
            archive.extractall(components / "site-packages")
    (evidence / "subjects.sha256").write_text("".join(subjects), encoding="ascii")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if (args.artifacts is None) != (args.output is None):
        parser.error("--artifacts and --output must be supplied together")
    run = json.loads(args.run.read_text(encoding="utf-8"))
    candidate = json.loads(CANDIDATE.read_text(encoding="utf-8"))
    verify_run(run, candidate, args.repository)
    if args.artifacts is not None:
        profile = json.loads(PROFILE.read_text(encoding="utf-8"))
        prepare(args.artifacts, args.output, run, profile, candidate)


if __name__ == "__main__":
    main()

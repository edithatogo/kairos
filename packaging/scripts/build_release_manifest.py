#!/usr/bin/env python3
"""Build the dry-run release manifest and SHA256SUMS from package inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 local developer fallback.
    tomllib = None


EXPECTED_ECOSYSTEMS = ("rust", "python", "r", "julia", "typescript", "csharp", "go")
FORBIDDEN_DRY_RUN_TEXT = re.compile(r"\b(publish|upload|login|token|credential|api[-_]?key)\b", re.IGNORECASE)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def require_string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise SystemExit(f"{label} must be a non-empty string")
    return value


def require_bool(value: object, label: str) -> bool:
    if not isinstance(value, bool):
        raise SystemExit(f"{label} must be a boolean")
    return value


def require_list(value: object, label: str) -> list:
    if not isinstance(value, list):
        raise SystemExit(f"{label} must be a list")
    return value


def resolve_repo_output(root: Path, relative_path: object, label: str) -> Path:
    path = require_string(relative_path, label)
    if Path(path).is_absolute():
        raise SystemExit(f"{label} output path must be repo-relative: {path}")
    normalized = Path(path)
    if ".." in normalized.parts:
        raise SystemExit(f"{label} output path must stay inside the repository: {path}")
    if normalized.parts[:1] != ("dist",):
        raise SystemExit(f"{label} output path must stay under ignored dist/: {path}")
    resolved = (root / normalized).resolve()
    try:
        resolved.relative_to(root.resolve())
    except ValueError as exc:
        raise SystemExit(f"{label} output path must stay inside the repository: {path}") from exc
    return resolved


def load_inventory(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("schema_version") != 1:
        raise SystemExit("release package inventory schema_version must be 1")
    if data.get("release_stage") != "r2-dry-run":
        raise SystemExit("release package inventory release_stage must be r2-dry-run")
    if data.get("production_publish_enabled") is not False:
        raise SystemExit("production_publish_enabled must remain false for dry-run packaging")
    output = data.get("output")
    if not isinstance(output, dict):
        raise SystemExit("release package inventory has no output contract")
    if set(output) != {"artifact_manifest", "checksum_manifest"}:
        raise SystemExit(
            "release package inventory output contract must only define artifact_manifest and checksum_manifest"
        )
    if output.get("artifact_manifest") != "dist/release-artifact-manifest.json":
        raise SystemExit("artifact_manifest output path must remain dist/release-artifact-manifest.json")
    if output.get("checksum_manifest") != "dist/SHA256SUMS":
        raise SystemExit("checksum_manifest output path must remain dist/SHA256SUMS")
    sequence = data.get("local_dry_run_sequence")
    if not isinstance(sequence, dict):
        raise SystemExit("release package inventory has no local_dry_run_sequence")
    if sequence.get("sequence_id") != "track15-r2-local-registry-package-dry-run":
        raise SystemExit("unexpected local_dry_run_sequence.sequence_id")
    if sequence.get("scope") != "offline inventory of the Rust workspace and binding package surfaces, plus local evidence generation only":
        raise SystemExit("unexpected local_dry_run_sequence.scope")
    if sequence.get("publish_manifests_allowed") is not False:
        raise SystemExit("publish manifests must remain disabled for the local dry-run sequence")
    steps = sequence.get("steps")
    if not isinstance(steps, list) or not steps:
        raise SystemExit("local_dry_run_sequence has no steps")
    expected_order = list(range(1, len(steps) + 1))
    actual_order = [step.get("order") for step in steps]
    if actual_order != expected_order:
        raise SystemExit("local_dry_run_sequence steps must be ordered from 1 without gaps")
    step_commands = []
    for step in steps:
        if not isinstance(step, dict):
            raise SystemExit("local_dry_run_sequence step must be an object")
        require_string(step.get("name"), "local_dry_run_sequence.step.name")
        command = require_string(step.get("command"), "local_dry_run_sequence.step.command")
        step_commands.append(command)
        if FORBIDDEN_DRY_RUN_TEXT.search(command):
            raise SystemExit(f"local dry-run step is not offline/non-publishing: {command}")
        require_bool(step.get("network_required"), "local_dry_run_sequence.step.network_required")
        if step.get("network_required") is not False:
            raise SystemExit(f"local dry-run step must not require network: {command}")
        writes = require_list(step.get("writes"), "local_dry_run_sequence.step.writes")
        for write in writes:
            require_string(write, "local_dry_run_sequence.step.write")
    if "python packaging/scripts/build_release_manifest.py --verify-existing" not in step_commands:
        raise SystemExit("local_dry_run_sequence must verify generated release evidence")
    surfaces = data.get("surfaces")
    if not isinstance(surfaces, list) or not surfaces:
        raise SystemExit("release package inventory has no surfaces")
    seen_ecosystems = set()
    for surface in surfaces:
        if not isinstance(surface, dict):
            raise SystemExit("surface entry must be an object")
        ecosystem = require_string(surface.get("ecosystem"), "surface.ecosystem")
        if ecosystem in seen_ecosystems:
            raise SystemExit(f"duplicate surface ecosystem: {ecosystem}")
        seen_ecosystems.add(ecosystem)
        require_string(surface.get("surface"), f"{ecosystem}.surface")
        require_string(surface.get("registry"), f"{ecosystem}.registry")
        require_string(surface.get("registry_mode"), f"{ecosystem}.registry_mode")
        require_string(surface.get("fallback"), f"{ecosystem}.fallback")
        commands = require_list(surface.get("dry_run_commands"), f"{ecosystem}.dry_run_commands")
        if not commands:
            raise SystemExit(f"{ecosystem} has no dry-run commands")
        for command in commands:
            command_text = require_string(command, f"{ecosystem}.dry_run_commands[]")
            if FORBIDDEN_DRY_RUN_TEXT.search(command_text):
                raise SystemExit(f"{ecosystem} dry-run commands must stay offline/non-publishing: {command_text}")
        manifests = require_list(surface.get("manifests"), f"{ecosystem}.manifests")
        if not manifests:
            raise SystemExit(f"{ecosystem} has no manifests")
        for manifest in manifests:
            if not isinstance(manifest, dict):
                raise SystemExit(f"{ecosystem} manifest entry must be an object")
            require_string(manifest.get("package"), f"{ecosystem}.manifests[].package")
            require_string(manifest.get("path"), f"{ecosystem}.manifests[].path")
    missing_ecosystems = set(EXPECTED_ECOSYSTEMS) - seen_ecosystems
    if missing_ecosystems:
        raise SystemExit(f"missing package ecosystems: {', '.join(sorted(missing_ecosystems))}")
    return data


def workspace_members(root: Path) -> set[str]:
    cargo_toml = (root / "Cargo.toml").read_text(encoding="utf-8")
    if tomllib is not None:
        workspace = tomllib.loads(cargo_toml)
        return {f"{member}/Cargo.toml" for member in workspace.get("workspace", {}).get("members", [])}

    match = re.search(r"(?ms)^\[workspace\]\s+.*?^members\s*=\s*\[(.*?)\]", cargo_toml)
    if not match:
        raise SystemExit("could not read Cargo workspace members")
    return {f"{member}/Cargo.toml" for member in re.findall(r'"([^"]+)"', match.group(1))}


def build(root: Path, inventory_path: Path, version: str, dry_run: str, check_only: bool) -> dict:
    inventory = load_inventory(inventory_path)
    output = inventory["output"]
    artifact_manifest = resolve_repo_output(root, output["artifact_manifest"], "artifact_manifest")
    checksum_manifest = resolve_repo_output(root, output["checksum_manifest"], "checksum_manifest")
    artifacts = []
    seen_paths = set()

    for surface in inventory["surfaces"]:
        ecosystem = surface["ecosystem"]
        for manifest in surface["manifests"]:
            rel = require_string(manifest["path"], f"{ecosystem}.manifests[].path")
            if rel in seen_paths:
                raise SystemExit(f"duplicate manifest path: {rel}")
            seen_paths.add(rel)
            path = root / rel
            if not path.is_file():
                raise SystemExit(f"missing package manifest: {rel}")
            artifacts.append(
                {
                    "ecosystem": ecosystem,
                    "surface": surface["surface"],
                    "package": manifest["package"],
                    "path": rel,
                    "registry": surface["registry"],
                    "registry_mode": surface["registry_mode"],
                    "sha256": sha256(path),
                }
            )

    rust_workspace_members = workspace_members(root)
    rust_paths = {
        manifest["path"]
        for surface in inventory["surfaces"]
        if surface["ecosystem"] == "rust"
        for manifest in surface.get("manifests", [])
        if manifest["path"] != "Cargo.toml"
    }
    if rust_paths != rust_workspace_members:
        missing = rust_workspace_members - rust_paths
        extra = rust_paths - rust_workspace_members
        details = []
        if missing:
            details.append(f"missing rust workspace members: {', '.join(sorted(missing))}")
        if extra:
            details.append(f"extra rust package manifests: {', '.join(sorted(extra))}")
        raise SystemExit("; ".join(details))

    ecosystems = {entry["ecosystem"] for entry in artifacts}
    missing = set(EXPECTED_ECOSYSTEMS) - ecosystems
    if missing:
        raise SystemExit(f"missing package ecosystems: {', '.join(sorted(missing))}")
    if len(artifacts) != len({(entry["ecosystem"], entry["path"]) for entry in artifacts}):
        raise SystemExit("artifact manifest must not contain duplicate ecosystem/path pairs")

    result = {
        "schema_version": 1,
        "version": version,
        "dry_run": dry_run,
        "release_stage": inventory.get("release_stage"),
        "production_publish_enabled": False,
        "source_inventory": inventory_path.relative_to(root).as_posix(),
        "local_dry_run_sequence": inventory["local_dry_run_sequence"],
        "artifact_count": len(artifacts),
        "artifacts": artifacts,
    }
    checksum_lines = "".join(f"{entry['sha256']}  {entry['path']}\n" for entry in artifacts)

    if not check_only:
        artifact_manifest.parent.mkdir(parents=True, exist_ok=True)
        artifact_manifest.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        checksum_manifest.write_text(checksum_lines, encoding="utf-8")
    return result


def verify_existing(root: Path, inventory_path: Path) -> dict:
    expected = build(root, inventory_path, version="0.0.0-verify", dry_run="true", check_only=True)
    inventory = load_inventory(inventory_path)
    artifact_manifest = resolve_repo_output(
        root,
        inventory["output"]["artifact_manifest"],
        "artifact_manifest",
    )
    checksum_manifest = resolve_repo_output(
        root,
        inventory["output"]["checksum_manifest"],
        "checksum_manifest",
    )
    if not artifact_manifest.is_file():
        raise SystemExit(f"missing generated artifact manifest: {artifact_manifest.relative_to(root)}")
    if not checksum_manifest.is_file():
        raise SystemExit(f"missing generated checksum manifest: {checksum_manifest.relative_to(root)}")

    actual = json.loads(artifact_manifest.read_text(encoding="utf-8"))
    for key in (
        "schema_version",
        "release_stage",
        "production_publish_enabled",
        "source_inventory",
        "local_dry_run_sequence",
        "artifact_count",
        "artifacts",
    ):
        if actual.get(key) != expected.get(key):
            raise SystemExit(f"generated artifact manifest drifted at {key}")
    require_string(actual.get("version"), "artifact_manifest.version")
    require_string(actual.get("dry_run"), "artifact_manifest.dry_run")

    expected_checksums = "".join(f"{entry['sha256']}  {entry['path']}\n" for entry in expected["artifacts"])
    actual_checksums = checksum_manifest.read_text(encoding="utf-8")
    if actual_checksums != expected_checksums:
        raise SystemExit("generated SHA256SUMS drifted from package inventory")

    return actual


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inventory", default="packaging/release-package-manifest.json")
    parser.add_argument("--version", default="0.0.0-dry-run")
    parser.add_argument("--dry-run", default="true")
    parser.add_argument("--check", action="store_true", help="validate inventory without writing dist files")
    parser.add_argument("--verify-existing", action="store_true", help="verify generated dist evidence without rewriting it")
    args = parser.parse_args()

    root = Path.cwd()
    inventory_path = root / args.inventory
    if args.verify_existing:
        result = verify_existing(root, inventory_path)
        print(f"verified {result['artifact_count']} generated package evidence entries")
    else:
        result = build(root, inventory_path, args.version, args.dry_run, args.check)
        print(f"validated {result['artifact_count']} package manifests across 7 ecosystems")


if __name__ == "__main__":
    main()

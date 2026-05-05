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


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def load_inventory(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("production_publish_enabled") is not False:
        raise SystemExit("production_publish_enabled must remain false for dry-run packaging")
    sequence = data.get("local_dry_run_sequence")
    if not isinstance(sequence, dict):
        raise SystemExit("release package inventory has no local_dry_run_sequence")
    if sequence.get("publish_manifests_allowed") is not False:
        raise SystemExit("publish manifests must remain disabled for the local dry-run sequence")
    steps = sequence.get("steps")
    if not isinstance(steps, list) or not steps:
        raise SystemExit("local_dry_run_sequence has no steps")
    expected_order = list(range(1, len(steps) + 1))
    actual_order = [step.get("order") for step in steps]
    if actual_order != expected_order:
        raise SystemExit("local_dry_run_sequence steps must be ordered from 1 without gaps")
    forbidden = re.compile(r"\b(publish|upload|login|token|credential|api[-_]?key)\b", re.IGNORECASE)
    for step in steps:
        command = step.get("command", "")
        if not isinstance(command, str) or not command:
            raise SystemExit("local_dry_run_sequence step has no command")
        if forbidden.search(command):
            raise SystemExit(f"local dry-run step is not offline/non-publishing: {command}")
        if step.get("network_required") is not False:
            raise SystemExit(f"local dry-run step must not require network: {command}")
    surfaces = data.get("surfaces")
    if not isinstance(surfaces, list) or not surfaces:
        raise SystemExit("release package inventory has no surfaces")
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
    output = inventory.get("output", {})
    artifact_manifest = root / output.get("artifact_manifest", "dist/release-artifact-manifest.json")
    checksum_manifest = root / output.get("checksum_manifest", "dist/SHA256SUMS")
    artifacts = []
    seen_paths = set()

    for surface in inventory["surfaces"]:
        ecosystem = surface["ecosystem"]
        commands = surface.get("dry_run_commands", [])
        if not commands:
            raise SystemExit(f"{ecosystem} has no dry-run commands")
        for manifest in surface.get("manifests", []):
            rel = manifest["path"]
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
    required = {"rust", "python", "r", "julia", "typescript", "csharp", "go"}
    missing = required - ecosystems
    if missing:
        raise SystemExit(f"missing package ecosystems: {', '.join(sorted(missing))}")

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

    if not check_only:
        artifact_manifest.parent.mkdir(parents=True, exist_ok=True)
        artifact_manifest.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        checksum_manifest.write_text(
            "".join(f"{entry['sha256']}  {entry['path']}\n" for entry in artifacts),
            encoding="utf-8",
        )
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inventory", default="packaging/release-package-manifest.json")
    parser.add_argument("--version", default="0.0.0-dry-run")
    parser.add_argument("--dry-run", default="true")
    parser.add_argument("--check", action="store_true", help="validate inventory without writing dist files")
    args = parser.parse_args()

    root = Path.cwd()
    result = build(root, root / args.inventory, args.version, args.dry_run, args.check)
    print(f"validated {result['artifact_count']} package manifests across 7 ecosystems")


if __name__ == "__main__":
    main()

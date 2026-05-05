#!/usr/bin/env python3
from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def require(path: str, contains: list[str] | None = None) -> None:
    target = ROOT / path
    if not target.exists():
        raise AssertionError(f"missing {path}")
    text = target.read_text(encoding="utf-8")
    for needle in contains or []:
        if needle not in text:
            raise AssertionError(f"{path} missing {needle!r}")


def run_json(command: list[str]) -> dict:
    completed = subprocess.run(
        command,
        check=True,
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    return json.loads(completed.stdout)


def validate_rendered_k8s_job() -> None:
    job = run_json(
        [
            sys.executable,
            "k8s/operator/kairoecs_operator.py",
            "--experiment",
            "k8s/samples/experiment.json",
        ]
    )

    assert job["apiVersion"] == "batch/v1"
    assert job["kind"] == "Job"
    spec = job["spec"]
    assert spec["parallelism"] == 2
    assert spec["completionMode"] == "Indexed"
    container = spec["template"]["spec"]["containers"][0]
    env = {entry["name"]: entry["value"] for entry in container["env"]}
    assert env["KAIRO_STORAGE_BACKEND"] == "filesystem"
    assert env["KAIRO_OUTPUT_URI"] == "/var/lib/kairo/output"
    assert env["KAIRO_CHECKPOINT_ENABLED"] == "true"


def validate_telemetry_plugin() -> None:
    validation_root = ROOT / "cloud" / "validation-work"
    validation_root.mkdir(parents=True, exist_ok=True)
    source = validation_root / "events.arrow"
    target = validation_root / "out"
    source.write_bytes(b"kairo-ecs-arrow-smoke\n")

    copied = run_json(
        [
            sys.executable,
            "docker/telemetry-plugin/cloud-output.py",
            "--input",
            str(source),
            "--destination",
            validation_root_uri(target),
        ]
    )
    copied_path = Path(copied["copied"])
    checksum_path = Path(copied["checksum"])
    assert copied_path.is_file()
    assert checksum_path.is_file()
    checksum_text = checksum_path.read_text(encoding="utf-8")
    assert copied["sha256"] in checksum_text
    assert copied_path.name in checksum_text

    provider = run_json(
        [
            sys.executable,
            "docker/telemetry-plugin/cloud-output.py",
            "--input",
            str(source),
            "--destination",
            "s3://kairo-smoke/runs/events.arrow",
        ]
    )
    manifest = json.loads(Path(provider["manifest"]).read_text(encoding="utf-8"))
    assert manifest["schema"] == "kairo.ecs.telemetry-upload.v1"
    assert manifest["provider"] == "s3"
    assert manifest["destination"] == "s3://kairo-smoke/runs/events.arrow"
    assert manifest["checksum_sha256"] == provider["sha256"]


def validation_root_uri(path: Path) -> str:
    return path.resolve().as_uri()


def main() -> int:
    validation_root = ROOT / "cloud" / "validation-work"
    if validation_root.exists():
        shutil.rmtree(validation_root)
    for path in [
        "cloud/gcp/batch-job.json",
        "cloud/gcp/batch-array.json",
        "cloud/azure/batch-job.json",
        "cloud/azure/batch-array.json",
        "k8s/samples/experiment.json",
    ]:
        json.loads((ROOT / path).read_text(encoding="utf-8"))

    require("docker/Dockerfile", ["kairo-ecs-cli", "ENTRYPOINT"])
    require("docker/entrypoint.sh", ["trap on_term TERM INT", "checkpoint-manifest.json"])
    require("k8s/crd/kairoecs-experiment.yaml", ["KairoECSExperiment", "checkpoint", "completedRuns"])
    require("k8s/operator/kairoecs_operator.py", ["render_job", "batch/v1"])
    require("cloud/aws/batch-job-definition.yaml", ["resourceRequirements", "KAIRO_OUTPUT_URI"])
    require("hpc/slurm/submit-experiment.sh", ["#SBATCH", "--signal=B:SIGTERM@120"])
    require("docs/cloud-hpc/slurm.md", ["checkpoint", "sbatch"])
    validate_rendered_k8s_job()
    validate_telemetry_plugin()
    if validation_root.exists():
        shutil.rmtree(validation_root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

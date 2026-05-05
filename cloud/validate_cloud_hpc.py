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


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def read_json(path: str) -> dict:
    return json.loads(read(path))


def run_json(command: list[str]) -> dict:
    completed = subprocess.run(
        command,
        check=True,
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    return json.loads(completed.stdout)


def validate_docker_surface() -> None:
    dockerfile = read("docker/Dockerfile")
    entrypoint = read("docker/entrypoint.sh")
    bake = read("docker/docker-bake.hcl")

    assert "FROM rust:" in dockerfile
    assert "FROM alpine:" in dockerfile
    assert "COPY --from=builder" in dockerfile
    assert "ENTRYPOINT" in dockerfile
    assert "USER kairo" in dockerfile
    assert "platforms = [\"linux/amd64\", \"linux/arm64\"]" in bake
    assert "trap on_term TERM INT" in entrypoint
    assert "checkpoint-manifest.json" in entrypoint
    assert ".tmp" in entrypoint and "mv \"$tmp_file\" \"$checkpoint_manifest\"" in entrypoint
    assert "resume --checkpoint" in entrypoint


def validate_cloud_manifests() -> None:
    aws_job = read("cloud/aws/batch-job-definition.yaml")
    aws_array = read("cloud/aws/batch-array-template.yaml")
    assert "jobDefinitionName: kairo-ecs-cli" in aws_job
    assert "type: container" in aws_job
    assert "platformCapabilities:" in aws_job and "FARGATE" in aws_job
    assert "resourceRequirements:" in aws_job
    assert "KAIRO_OUTPUT_URI" in aws_job
    assert "KAIRO_CHECKPOINT_DIR" in aws_job
    assert "arrayProperties:" in aws_array
    assert "${AWS_BATCH_JOB_ARRAY_INDEX}" in aws_array

    gcp_job = read_json("cloud/gcp/batch-job.json")
    gcp_array = read_json("cloud/gcp/batch-array.json")
    task_group = gcp_job["taskGroups"][0]
    runnable = task_group["taskSpec"]["runnables"][0]["container"]
    assert runnable["imageUri"] == "${KAIRO_IMAGE}"
    assert runnable["commands"][:2] == ["run", "--scenario"]
    variables = task_group["taskSpec"]["environment"]["variables"]
    assert variables["KAIRO_OUTPUT_URI"] == "${KAIRO_OUTPUT_URI}"
    assert variables["KAIRO_CHECKPOINT_DIR"] == "/tmp/kairo/checkpoints"
    assert gcp_array["taskGroups"][0]["taskSpec"]["runnables"][0]["container"]["commands"][2].endswith("variant-${BATCH_TASK_INDEX}.yaml")

    azure_job = read_json("cloud/azure/batch-job.json")
    azure_array = read_json("cloud/azure/batch-array.json")
    assert azure_job["id"] == "kairo-ecs-job"
    assert azure_job["onAllTasksComplete"] == "terminateJob"
    assert azure_array["metadata"][0]["name"] == "sweepSize"
    submitter = read("cloud/azure/submit-experiment.ps1")
    assert "az batch job create" in submitter
    assert "az batch task create" in submitter
    assert "KAIRO_OUTPUT_URI=$OutputUri" in submitter


def validate_k8s_manifests() -> None:
    crd = read("k8s/crd/kairoecs-experiment.yaml")
    assert "apiVersion: apiextensions.k8s.io/v1" in crd
    assert "kind: CustomResourceDefinition" in crd
    assert "KairoECSExperiment" in crd
    assert "required: [image, scenarioRef, storage]" in crd
    assert "enum: [s3, gcs, azure, filesystem]" in crd
    assert "subresources:" in crd and "status: {}" in crd

    sample = read_json("k8s/samples/experiment.json")
    assert sample["kind"] == "KairoECSExperiment"
    assert sample["spec"]["parallelism"] >= 1
    assert sample["spec"]["storage"]["backend"] in {"filesystem", "s3", "gcs", "azure"}
    validate_rendered_k8s_job()


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


def validate_slurm_scripts() -> None:
    single = read("hpc/slurm/submit-experiment.sh")
    sweep = read("hpc/slurm/submit-sweep.sh")
    resume = read("hpc/slurm/resume.sh")
    for path, text in {
        "hpc/slurm/submit-experiment.sh": single,
        "hpc/slurm/submit-sweep.sh": sweep,
        "hpc/slurm/resume.sh": resume,
    }.items():
        assert "set -euo pipefail" in text, path

    assert "#SBATCH --signal=B:SIGTERM@120" in single
    assert 'KAIRO_CHECKPOINT_DIR="\\${KAIRO_CHECKPOINT_DIR:-${TMPDIR:-/tmp}/kairo/checkpoints}"' in single
    assert "kairo-ecs-cli checkpoint" in single
    assert "#SBATCH --array=0-${last_index}" in sweep
    assert "#SBATCH --signal=B:SIGTERM@120" in sweep
    assert "SLURM_ARRAY_TASK_ID" in sweep
    assert "kairo-ecs-cli resume --checkpoint" in resume

    bash = shutil.which("bash")
    if bash:
        probe = subprocess.run([bash, "--version"], cwd=ROOT, text=True, capture_output=True)
        if probe.returncode != 0:
            return
        for script in [
            "hpc/slurm/submit-experiment.sh",
            "hpc/slurm/submit-sweep.sh",
            "hpc/slurm/resume.sh",
            "docker/entrypoint.sh",
            "cloud/aws/submit-experiment.sh",
            "cloud/gcp/submit-experiment.sh",
        ]:
            subprocess.run([bash, "-n", script], cwd=ROOT, check=True)


def validate_policy_docs() -> None:
    policy = read("docs/cloud-hpc/checkpoint-spot-policy.md")
    assert "Offline validator scope" in policy
    assert "Live provider validation" in policy
    assert "local or shared POSIX filesystem path" in policy
    assert "does not prove" in policy
    for path in [
        "docs/cloud-hpc/aws-batch.md",
        "docs/cloud-hpc/gcp-batch.md",
        "docs/cloud-hpc/azure-batch.md",
        "docs/cloud-hpc/slurm.md",
    ]:
        text = read(path)
        assert "Offline validation" in text, path
        assert "Live validation" in text, path


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
    validate_docker_surface()
    validate_cloud_manifests()
    validate_k8s_manifests()
    validate_slurm_scripts()
    validate_policy_docs()
    validate_telemetry_plugin()
    if validation_root.exists():
        shutil.rmtree(validation_root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

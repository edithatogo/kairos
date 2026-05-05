#!/usr/bin/env python3
from __future__ import annotations

import json
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


def main() -> int:
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

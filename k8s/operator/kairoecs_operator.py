#!/usr/bin/env python3
"""Offline-renderable Kubernetes Job skeleton for KairoECS experiments."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


VALID_STORAGE_BACKENDS = {"filesystem", "s3", "gcs", "azure"}


def validate_experiment(experiment: dict) -> None:
    if experiment.get("kind") != "KairoECSExperiment":
        raise ValueError("experiment kind must be KairoECSExperiment")
    spec = experiment.get("spec")
    if not isinstance(spec, dict):
        raise ValueError("experiment spec must be an object")
    if not str(spec.get("image", "")).strip():
        raise ValueError("spec.image must not be empty")
    parallelism = int(spec.get("parallelism", 1))
    if parallelism < 1:
        raise ValueError("spec.parallelism must be greater than zero")
    storage = spec.get("storage")
    if not isinstance(storage, dict):
        raise ValueError("spec.storage must be an object")
    if storage.get("backend") not in VALID_STORAGE_BACKENDS:
        raise ValueError("spec.storage.backend must be one of azure, filesystem, gcs, s3")
    if not str(storage.get("path", "")).strip():
        raise ValueError("spec.storage.path must not be empty")


def render_job(experiment: dict) -> dict:
    validate_experiment(experiment)
    metadata = experiment.get("metadata", {})
    spec = experiment["spec"]
    name = metadata.get("name", "kairo-experiment")
    parallelism = int(spec.get("parallelism", 1))
    storage = spec["storage"]
    return {
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": {"name": f"{name}-run"},
        "spec": {
            "parallelism": parallelism,
            "completions": parallelism,
            "completionMode": "Indexed",
            "template": {
                "spec": {
                    "restartPolicy": "Never",
                    "containers": [
                        {
                            "name": "kairo-ecs-cli",
                            "image": spec["image"],
                            "imagePullPolicy": spec.get("imagePullPolicy", "IfNotPresent"),
                            "args": ["run", "--scenario", "/scenario/scenario.yaml"],
                            "env": [
                                {"name": "KAIRO_STORAGE_BACKEND", "value": storage["backend"]},
                                {"name": "KAIRO_OUTPUT_URI", "value": storage["path"]},
                                {"name": "KAIRO_CHECKPOINT_ENABLED", "value": str(spec.get("checkpoint", {}).get("enabled", True)).lower()},
                            ],
                            "resources": spec.get("resources", {}),
                        }
                    ],
                }
            },
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--experiment", required=True)
    parser.add_argument("--output")
    args = parser.parse_args()

    experiment = json.loads(Path(args.experiment).read_text(encoding="utf-8"))
    job = render_job(experiment)
    rendered = json.dumps(job, indent=2, sort_keys=True) + "\n"
    if args.output:
        Path(args.output).write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

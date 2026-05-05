#!/usr/bin/env python3
"""Offline-renderable Kubernetes Job skeleton for KairoECS experiments."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def render_job(experiment: dict) -> dict:
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

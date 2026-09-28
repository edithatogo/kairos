#!/usr/bin/env python3
"""Offline-renderable Kubernetes Job skeleton for KairoECS experiments."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path, PurePosixPath

VALID_STORAGE_BACKENDS = {"filesystem", "s3", "gcs", "azure"}
CONFIG_MAP_KEY = re.compile(r"[A-Za-z0-9._-]+\Z")
DNS_LABEL = re.compile(r"[a-z0-9](?:[-a-z0-9]*[a-z0-9])?\Z")


def validate_scenario_key(key: object) -> str:
    """Require one portable ConfigMap key and one file directly under /scenario."""
    if not isinstance(key, str):
        raise TypeError("spec.scenarioRef.key must be a string")
    if key.startswith("/"):
        raise ValueError("spec.scenarioRef.key must not be an absolute path")
    if ".." in key.split("/"):
        raise ValueError(
            "spec.scenarioRef.key must not contain path traversal components"
        )
    if "\\" in key:
        raise ValueError("spec.scenarioRef.key must not contain backslashes")
    if "%" in key:
        raise ValueError("spec.scenarioRef.key must not use URL-encoded paths")
    if key in {"", ".", ".."} or not CONFIG_MAP_KEY.fullmatch(key):
        raise ValueError(
            "spec.scenarioRef.key must be a single safe ConfigMap filename"
        )

    scenario_dir = PurePosixPath("/scenario")
    scenario_path = scenario_dir / key
    if scenario_path.parent != scenario_dir or scenario_path.name != key:
        raise ValueError("spec.scenarioRef.key must stay inside /scenario")
    return key


def is_dns_subdomain(value: object) -> bool:
    if not isinstance(value, str) or len(value) > 253:
        return False
    labels = value.split(".")
    return all(len(label) <= 63 and DNS_LABEL.fullmatch(label) for label in labels)


def validate_experiment(experiment: dict) -> None:
    if experiment.get("kind") != "KairoECSExperiment":
        raise ValueError("experiment kind must be KairoECSExperiment")
    spec = experiment.get("spec")
    if not isinstance(spec, dict):
        raise TypeError("experiment spec must be an object")
    if not str(spec.get("image", "")).strip():
        raise ValueError("spec.image must not be empty")
    parallelism = int(spec.get("parallelism", 1))
    if parallelism < 1:
        raise ValueError("spec.parallelism must be greater than zero")
    storage = spec.get("storage")
    if not isinstance(storage, dict):
        raise TypeError("spec.storage must be an object")
    if storage.get("backend") not in VALID_STORAGE_BACKENDS:
        raise ValueError(
            "spec.storage.backend must be one of azure, filesystem, gcs, s3"
        )
    if not str(storage.get("path", "")).strip():
        raise ValueError("spec.storage.path must not be empty")
    scenario_ref = spec.get("scenarioRef")
    if not isinstance(scenario_ref, dict):
        raise TypeError("spec.scenarioRef must be an object")
    validate_scenario_key(scenario_ref.get("key", "scenario.yaml"))
    config_map_name = scenario_ref.get("configMapName")
    if config_map_name is not None and not is_dns_subdomain(config_map_name):
        raise ValueError("spec.scenarioRef.configMapName must be a DNS subdomain")
    if (
        not scenario_ref.get("configMapName")
        and not str(scenario_ref.get("inline", "")).strip()
    ):
        raise ValueError(
            "spec.scenarioRef must provide configMapName or inline scenario content"
        )


def render_job(experiment: dict) -> dict:
    validate_experiment(experiment)
    metadata = experiment.get("metadata", {})
    spec = experiment["spec"]
    name = metadata.get("name", "kairo-experiment")
    parallelism = int(spec.get("parallelism", 1))
    storage = spec["storage"]
    scenario_ref = spec.get("scenarioRef", {})
    scenario_key = validate_scenario_key(scenario_ref.get("key", "scenario.yaml"))
    scenario_mount = f"/scenario/{scenario_key}"
    volumes = []
    volume_mounts = []
    init_containers = []
    if scenario_ref.get("configMapName"):
        volumes.append(
            {
                "name": "scenario",
                "configMap": {
                    "name": scenario_ref["configMapName"],
                    "items": [{"key": scenario_key, "path": scenario_key}],
                },
            }
        )
        volume_mounts.append(
            {"name": "scenario", "mountPath": "/scenario", "readOnly": True}
        )
    elif scenario_ref.get("inline"):
        volumes.append({"name": "scenario", "emptyDir": {}})
        volume_mounts.append(
            {"name": "scenario", "mountPath": "/scenario", "readOnly": True}
        )
        init_containers.append(
            {
                "name": "write-inline-scenario",
                "image": "busybox:1.36",
                "command": [
                    "sh",
                    "-c",
                    'printf \'%s\' "$KAIRO_INLINE_SCENARIO" > "$KAIRO_SCENARIO_DIR/$KAIRO_SCENARIO_KEY"',
                ],
                "env": [
                    {"name": "KAIRO_INLINE_SCENARIO", "value": scenario_ref["inline"]},
                    {"name": "KAIRO_SCENARIO_KEY", "value": scenario_key},
                    {"name": "KAIRO_SCENARIO_DIR", "value": "/scenario"},
                ],
                "volumeMounts": [{"name": "scenario", "mountPath": "/scenario"}],
            }
        )
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
                    "initContainers": init_containers,
                    "containers": [
                        {
                            "name": "kairo-ecs-cli",
                            "image": spec["image"],
                            "imagePullPolicy": spec.get(
                                "imagePullPolicy", "IfNotPresent"
                            ),
                            "args": [
                                "run",
                                "--scenario",
                                scenario_mount,
                                "--output",
                                storage["path"],
                            ],
                            "env": [
                                {
                                    "name": "KAIRO_STORAGE_BACKEND",
                                    "value": storage["backend"],
                                },
                                {"name": "KAIRO_OUTPUT_URI", "value": storage["path"]},
                                {
                                    "name": "KAIRO_CHECKPOINT_ENABLED",
                                    "value": str(
                                        spec.get("checkpoint", {}).get("enabled", True)
                                    ).lower(),
                                },
                            ],
                            "volumeMounts": volume_mounts,
                            "resources": spec.get("resources", {}),
                        }
                    ],
                    "volumes": volumes,
                }
            },
        },
    }


def render_status_patch(experiment: dict, phase: str = "Rendered") -> dict:
    metadata = experiment.get("metadata", {})
    return {
        "apiVersion": experiment.get("apiVersion", "kairo.ecs/v1alpha1"),
        "kind": experiment.get("kind", "KairoECSExperiment"),
        "metadata": {"name": metadata.get("name", "kairo-experiment")},
        "status": {
            "phase": phase,
            "completedRuns": 0,
            "failedRuns": 0,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--experiment", required=True)
    parser.add_argument("--output")
    parser.add_argument(
        "--status",
        action="store_true",
        help="render the offline status patch instead of the Job",
    )
    args = parser.parse_args()

    experiment = json.loads(Path(args.experiment).read_text(encoding="utf-8"))
    rendered_object = (
        render_status_patch(experiment) if args.status else render_job(experiment)
    )
    rendered = json.dumps(rendered_object, indent=2, sort_keys=True) + "\n"
    if args.output:
        Path(args.output).write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

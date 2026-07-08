import pytest

from kairoecs_operator import render_job, validate_experiment

def test_render_job_with_configmap():
    experiment = {
        "kind": "KairoECSExperiment",
        "metadata": {
            "name": "test-exp"
        },
        "spec": {
            "image": "kairo:latest",
            "parallelism": 2,
            "storage": {
                "backend": "s3",
                "path": "s3://bucket/test"
            },
            "scenarioRef": {
                "configMapName": "scenario-config",
                "key": "custom-scenario.yaml"
            },
            "resources": {
                "requests": {"cpu": "1", "memory": "1Gi"}
            },
            "checkpoint": {"enabled": False}
        }
    }

    job = render_job(experiment)

    assert job["apiVersion"] == "batch/v1"
    assert job["kind"] == "Job"
    assert job["metadata"]["name"] == "test-exp-run"
    assert job["spec"]["parallelism"] == 2
    assert job["spec"]["completions"] == 2

    template_spec = job["spec"]["template"]["spec"]
    assert template_spec["restartPolicy"] == "Never"

    # Check container details
    assert len(template_spec["containers"]) == 1
    container = template_spec["containers"][0]
    assert container["image"] == "kairo:latest"
    assert container["resources"] == {"requests": {"cpu": "1", "memory": "1Gi"}}

    # Check env vars
    env = {e["name"]: e["value"] for e in container["env"]}
    assert env["KAIRO_STORAGE_BACKEND"] == "s3"
    assert env["KAIRO_OUTPUT_URI"] == "s3://bucket/test"
    assert env["KAIRO_CHECKPOINT_ENABLED"] == "false"

    # Check args
    assert "--scenario" in container["args"]
    assert "/scenario/custom-scenario.yaml" in container["args"]
    assert "--output" in container["args"]
    assert "s3://bucket/test" in container["args"]

    # Check volumes and mounts
    assert len(template_spec["volumes"]) == 1
    assert template_spec["volumes"][0]["name"] == "scenario"
    assert template_spec["volumes"][0]["configMap"]["name"] == "scenario-config"

    assert len(container["volumeMounts"]) == 1
    assert container["volumeMounts"][0]["name"] == "scenario"
    assert container["volumeMounts"][0]["mountPath"] == "/scenario"

def test_render_job_with_inline_scenario():
    experiment = {
        "kind": "KairoECSExperiment",
        "metadata": {
            "name": "inline-exp"
        },
        "spec": {
            "image": "kairo:latest",
            "storage": {
                "backend": "filesystem",
                "path": "/mnt/data"
            },
            "scenarioRef": {
                "inline": "scenario-content-here"
            }
        }
    }

    job = render_job(experiment)

    template_spec = job["spec"]["template"]["spec"]

    # Check initContainers for inline scenario
    assert len(template_spec.get("initContainers", [])) == 1
    init_container = template_spec["initContainers"][0]
    assert init_container["name"] == "write-inline-scenario"

    env = {e["name"]: e["value"] for e in init_container["env"]}
    assert env["KAIRO_INLINE_SCENARIO"] == "scenario-content-here"

    # Default scenario key is 'scenario.yaml'
    assert "/scenario/scenario.yaml" in init_container["command"][2]

    # Default parallelism and completions
    assert job["spec"]["parallelism"] == 1
    assert job["spec"]["completions"] == 1

    # Default checkpoint enabled
    container = template_spec["containers"][0]
    env = {e["name"]: e["value"] for e in container["env"]}
    assert env["KAIRO_CHECKPOINT_ENABLED"] == "true"

def test_render_job_validation_errors():
    base_experiment = {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "kairo:latest",
            "storage": {
                "backend": "s3",
                "path": "s3://test"
            },
            "scenarioRef": {
                "inline": "test"
            }
        }
    }

    # Test invalid kind
    invalid_kind = base_experiment.copy()
    invalid_kind["kind"] = "Pod"
    with pytest.raises(ValueError, match="experiment kind must be KairoECSExperiment"):
        render_job(invalid_kind)

    # Test empty image
    empty_image = base_experiment.copy()
    empty_image["spec"] = base_experiment["spec"].copy()
    empty_image["spec"]["image"] = "   "
    with pytest.raises(ValueError, match="spec.image must not be empty"):
        render_job(empty_image)

    # Test invalid storage backend
    invalid_storage = base_experiment.copy()
    invalid_storage["spec"] = base_experiment["spec"].copy()
    invalid_storage["spec"]["storage"] = base_experiment["spec"]["storage"].copy()
    invalid_storage["spec"]["storage"]["backend"] = "invalid"
    with pytest.raises(ValueError, match="spec.storage.backend must be one of azure, filesystem, gcs, s3"):
        render_job(invalid_storage)

    # Test missing scenarioRef info
    missing_ref = base_experiment.copy()
    missing_ref["spec"] = base_experiment["spec"].copy()
    missing_ref["spec"]["scenarioRef"] = {}
    with pytest.raises(ValueError, match="spec.scenarioRef must provide configMapName or inline scenario content"):
        render_job(missing_ref)

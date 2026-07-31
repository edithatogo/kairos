import pytest
from kairoecs_operator import render_job, validate_experiment


def test_render_job_inline_scenario_injection():
    # A malicious key intended to execute commands
    malicious_key = "scenario.yaml"

    experiment = {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "my-image:latest",
            "storage": {"backend": "s3", "path": "s3://bucket/out"},
            "scenarioRef": {"key": malicious_key, "inline": "scenario content"},
        },
    }

    # Should not raise
    validate_experiment(experiment)

    job = render_job(experiment)

    init_containers = job["spec"]["template"]["spec"]["initContainers"]
    write_container = next(
        c for c in init_containers if c["name"] == "write-inline-scenario"
    )

    command = write_container["command"]
    assert command[0] == "sh"
    assert command[1] == "-c"

    # The command should use an env var rather than f-string formatting
    assert "$KAIRO_SCENARIO_KEY" in command[2]

    envs = {env["name"]: env["value"] for env in write_container["env"]}
    assert "KAIRO_SCENARIO_KEY" in envs
    assert envs["KAIRO_SCENARIO_KEY"] == malicious_key


def test_validate_experiment_path_traversal():
    experiment = {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "test",
            "storage": {"backend": "s3", "path": "x"},
            "scenarioRef": {"key": "../../../etc/passwd", "inline": "foo"},
        },
    }
    with pytest.raises(ValueError, match="path traversal components"):
        validate_experiment(experiment)


def test_validate_experiment_absolute_path():
    experiment = {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "test",
            "storage": {"backend": "s3", "path": "x"},
            "scenarioRef": {"key": "/etc/passwd", "inline": "foo"},
        },
    }
    with pytest.raises(ValueError, match="absolute path"):
        validate_experiment(experiment)

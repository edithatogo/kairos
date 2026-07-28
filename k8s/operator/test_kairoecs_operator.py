import pytest
from kairoecs_operator import validate_experiment


def test_validate_experiment_success():
    """Happy path: a fully valid experiment should not raise any exceptions."""
    valid_experiment = {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "my-image:latest",
            "parallelism": 2,
            "storage": {"backend": "s3", "path": "s3://my-bucket/output"},
            "scenarioRef": {"configMapName": "my-scenario-config"},
        },
    }
    # Should not raise
    validate_experiment(valid_experiment)


@pytest.mark.parametrize(
    "experiment_update, expected_error",
    [
        ({"kind": "OtherKind"}, "experiment kind must be KairoECSExperiment"),
        ({"spec": None}, "experiment spec must be an object"),
        ({"spec": "not a dict"}, "experiment spec must be an object"),
        (
            {
                "spec": {
                    "image": "",
                    "storage": {"backend": "s3", "path": "p"},
                    "scenarioRef": {"inline": "s"},
                }
            },
            "spec.image must not be empty",
        ),
        (
            {
                "spec": {
                    "image": "  ",
                    "storage": {"backend": "s3", "path": "p"},
                    "scenarioRef": {"inline": "s"},
                }
            },
            "spec.image must not be empty",
        ),
        (
            {
                "spec": {
                    "image": "img",
                    "parallelism": 0,
                    "storage": {"backend": "s3", "path": "p"},
                    "scenarioRef": {"inline": "s"},
                }
            },
            "spec.parallelism must be greater than zero",
        ),
        (
            {
                "spec": {
                    "image": "img",
                    "parallelism": -1,
                    "storage": {"backend": "s3", "path": "p"},
                    "scenarioRef": {"inline": "s"},
                }
            },
            "spec.parallelism must be greater than zero",
        ),
        (
            {"spec": {"image": "img", "storage": None, "scenarioRef": {"inline": "s"}}},
            "spec.storage must be an object",
        ),
        (
            {
                "spec": {
                    "image": "img",
                    "storage": {"backend": "unknown", "path": "p"},
                    "scenarioRef": {"inline": "s"},
                }
            },
            "spec.storage.backend must be one of azure, filesystem, gcs, s3",
        ),
        (
            {
                "spec": {
                    "image": "img",
                    "storage": {"backend": "s3", "path": ""},
                    "scenarioRef": {"inline": "s"},
                }
            },
            "spec.storage.path must not be empty",
        ),
        (
            {
                "spec": {
                    "image": "img",
                    "storage": {"backend": "s3", "path": "p"},
                    "scenarioRef": None,
                }
            },
            "spec.scenarioRef must be an object",
        ),
        (
            {
                "spec": {
                    "image": "img",
                    "storage": {"backend": "s3", "path": "p"},
                    "scenarioRef": {},
                }
            },
            "spec.scenarioRef must provide configMapName or inline scenario content",
        ),
        (
            {
                "spec": {
                    "image": "img",
                    "storage": {"backend": "s3", "path": "p"},
                    "scenarioRef": {"inline": "  "},
                }
            },
            "spec.scenarioRef must provide configMapName or inline scenario content",
        ),
    ],
)
def test_validate_experiment_failures(experiment_update, expected_error):
    # Base valid experiment
    experiment = {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "my-image:latest",
            "parallelism": 1,
            "storage": {"backend": "s3", "path": "s3://my-bucket/output"},
            "scenarioRef": {"configMapName": "my-scenario-config"},
        },
    }

    # Update the base experiment with the specific failure condition
    experiment.update(experiment_update)

    with pytest.raises(ValueError, match=expected_error):
        validate_experiment(experiment)

import pytest
from kairoecs_operator import validate_experiment

def test_validate_experiment_valid():
    experiment = {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "my-image:latest",
            "parallelism": 1,
            "storage": {
                "backend": "filesystem",
                "path": "/output"
            },
            "scenarioRef": {
                "configMapName": "my-scenario"
            }
        }
    }
    # Should not raise any exception
    validate_experiment(experiment)

def test_validate_experiment_valid_inline():
    experiment = {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "my-image:latest",
            "parallelism": 1,
            "storage": {
                "backend": "filesystem",
                "path": "/output"
            },
            "scenarioRef": {
                "inline": "scenario data"
            }
        }
    }
    # Should not raise any exception
    validate_experiment(experiment)

def test_validate_experiment_invalid_kind():
    with pytest.raises(ValueError, match="experiment kind must be KairoECSExperiment"):
        validate_experiment({"kind": "Deployment"})

def test_validate_experiment_missing_spec():
    with pytest.raises(ValueError, match="experiment spec must be an object"):
        validate_experiment({"kind": "KairoECSExperiment"})

def test_validate_experiment_invalid_spec():
    with pytest.raises(ValueError, match="experiment spec must be an object"):
        validate_experiment({"kind": "KairoECSExperiment", "spec": "invalid"})

def test_validate_experiment_empty_image():
    with pytest.raises(ValueError, match="spec.image must not be empty"):
        validate_experiment({
            "kind": "KairoECSExperiment",
            "spec": {
                "image": "  "
            }
        })

def test_validate_experiment_invalid_parallelism():
    with pytest.raises(ValueError, match="spec.parallelism must be greater than zero"):
        validate_experiment({
            "kind": "KairoECSExperiment",
            "spec": {
                "image": "my-image",
                "parallelism": 0
            }
        })

def test_validate_experiment_missing_storage():
    with pytest.raises(ValueError, match="spec.storage must be an object"):
        validate_experiment({
            "kind": "KairoECSExperiment",
            "spec": {
                "image": "my-image",
                "parallelism": 1
            }
        })

def test_validate_experiment_invalid_storage_backend():
    with pytest.raises(ValueError, match="spec.storage.backend must be one of azure, filesystem, gcs, s3"):
        validate_experiment({
            "kind": "KairoECSExperiment",
            "spec": {
                "image": "my-image",
                "parallelism": 1,
                "storage": {
                    "backend": "unsupported",
                    "path": "/out"
                }
            }
        })

def test_validate_experiment_empty_storage_path():
    with pytest.raises(ValueError, match="spec.storage.path must not be empty"):
        validate_experiment({
            "kind": "KairoECSExperiment",
            "spec": {
                "image": "my-image",
                "parallelism": 1,
                "storage": {
                    "backend": "filesystem",
                    "path": ""
                }
            }
        })

def test_validate_experiment_missing_scenario_ref():
    with pytest.raises(ValueError, match="spec.scenarioRef must be an object"):
        validate_experiment({
            "kind": "KairoECSExperiment",
            "spec": {
                "image": "my-image",
                "parallelism": 1,
                "storage": {
                    "backend": "filesystem",
                    "path": "/out"
                }
            }
        })

def test_validate_experiment_missing_scenario_content():
    with pytest.raises(ValueError, match="spec.scenarioRef must provide configMapName or inline scenario content"):
        validate_experiment({
            "kind": "KairoECSExperiment",
            "spec": {
                "image": "my-image",
                "parallelism": 1,
                "storage": {
                    "backend": "filesystem",
                    "path": "/out"
                },
                "scenarioRef": {}
            }
        })

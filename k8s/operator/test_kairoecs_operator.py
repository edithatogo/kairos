import pytest

from kairoecs_operator import validate_experiment

def valid_experiment():
    return {
        "kind": "KairoECSExperiment",
        "metadata": {"name": "test-exp"},
        "spec": {
            "image": "my-image:latest",
            "parallelism": 1,
            "storage": {
                "backend": "filesystem",
                "path": "/output"
            },
            "scenarioRef": {
                "configMapName": "my-scenario-cm"
            }
        }
    }

def test_validate_experiment_valid():
    # Should not raise an exception
    validate_experiment(valid_experiment())

def test_validate_experiment_invalid_kind():
    exp = valid_experiment()
    exp["kind"] = "InvalidKind"
    with pytest.raises(ValueError, match="experiment kind must be KairoECSExperiment"):
        validate_experiment(exp)

def test_validate_experiment_missing_spec():
    exp = valid_experiment()
    del exp["spec"]
    with pytest.raises(ValueError, match="experiment spec must be an object"):
        validate_experiment(exp)

def test_validate_experiment_empty_image():
    exp = valid_experiment()
    exp["spec"]["image"] = "   "
    with pytest.raises(ValueError, match="spec.image must not be empty"):
        validate_experiment(exp)

def test_validate_experiment_invalid_parallelism():
    exp = valid_experiment()
    exp["spec"]["parallelism"] = 0
    with pytest.raises(ValueError, match="spec.parallelism must be greater than zero"):
        validate_experiment(exp)

def test_validate_experiment_missing_storage():
    exp = valid_experiment()
    del exp["spec"]["storage"]
    with pytest.raises(ValueError, match="spec.storage must be an object"):
        validate_experiment(exp)

def test_validate_experiment_invalid_storage_backend():
    exp = valid_experiment()
    exp["spec"]["storage"]["backend"] = "invalid"
    with pytest.raises(ValueError, match="spec.storage.backend must be one of azure, filesystem, gcs, s3"):
        validate_experiment(exp)

def test_validate_experiment_empty_storage_path():
    exp = valid_experiment()
    exp["spec"]["storage"]["path"] = ""
    with pytest.raises(ValueError, match="spec.storage.path must not be empty"):
        validate_experiment(exp)

def test_validate_experiment_missing_scenario_ref():
    exp = valid_experiment()
    del exp["spec"]["scenarioRef"]
    with pytest.raises(ValueError, match="spec.scenarioRef must be an object"):
        validate_experiment(exp)

def test_validate_experiment_missing_scenario_ref_content():
    exp = valid_experiment()
    exp["spec"]["scenarioRef"] = {}
    with pytest.raises(ValueError, match="spec.scenarioRef must provide configMapName or inline scenario content"):
        validate_experiment(exp)

def test_validate_experiment_inline_scenario_ref():
    exp = valid_experiment()
    exp["spec"]["scenarioRef"] = {"inline": "some content"}
    # Should not raise an exception
    validate_experiment(exp)

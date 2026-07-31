import pytest
from kairoecs_operator import validate_experiment

@pytest.fixture
def valid_experiment():
    return {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "kairo-image:latest",
            "parallelism": 1,
            "storage": {
                "backend": "s3",
                "path": "s3://bucket/path"
            },
            "scenarioRef": {
                "configMapName": "test-scenario"
            }
        }
    }

def test_validate_experiment_valid(valid_experiment):
    # Should not raise any exception
    validate_experiment(valid_experiment)

def test_validate_experiment_invalid_kind(valid_experiment):
    valid_experiment["kind"] = "InvalidKind"
    with pytest.raises(ValueError, match="experiment kind must be KairoECSExperiment"):
        validate_experiment(valid_experiment)

def test_validate_experiment_missing_kind():
    with pytest.raises(ValueError, match="experiment kind must be KairoECSExperiment"):
        validate_experiment({})

def test_validate_experiment_invalid_spec(valid_experiment):
    valid_experiment["spec"] = "not a dict"
    with pytest.raises(ValueError, match="experiment spec must be an object"):
        validate_experiment(valid_experiment)

def test_validate_experiment_missing_spec(valid_experiment):
    del valid_experiment["spec"]
    with pytest.raises(ValueError, match="experiment spec must be an object"):
        validate_experiment(valid_experiment)

def test_validate_experiment_empty_image(valid_experiment):
    valid_experiment["spec"]["image"] = "   "
    with pytest.raises(ValueError, match="spec.image must not be empty"):
        validate_experiment(valid_experiment)

def test_validate_experiment_missing_image(valid_experiment):
    del valid_experiment["spec"]["image"]
    with pytest.raises(ValueError, match="spec.image must not be empty"):
        validate_experiment(valid_experiment)

def test_validate_experiment_invalid_parallelism(valid_experiment):
    valid_experiment["spec"]["parallelism"] = 0
    with pytest.raises(ValueError, match="spec.parallelism must be greater than zero"):
        validate_experiment(valid_experiment)

    valid_experiment["spec"]["parallelism"] = -5
    with pytest.raises(ValueError, match="spec.parallelism must be greater than zero"):
        validate_experiment(valid_experiment)

def test_validate_experiment_invalid_storage(valid_experiment):
    valid_experiment["spec"]["storage"] = "not a dict"
    with pytest.raises(ValueError, match="spec.storage must be an object"):
        validate_experiment(valid_experiment)

def test_validate_experiment_missing_storage(valid_experiment):
    del valid_experiment["spec"]["storage"]
    with pytest.raises(ValueError, match="spec.storage must be an object"):
        validate_experiment(valid_experiment)

def test_validate_experiment_invalid_storage_backend(valid_experiment):
    valid_experiment["spec"]["storage"]["backend"] = "invalid"
    with pytest.raises(ValueError, match="spec.storage.backend must be one of azure, filesystem, gcs, s3"):
        validate_experiment(valid_experiment)

def test_validate_experiment_empty_storage_path(valid_experiment):
    valid_experiment["spec"]["storage"]["path"] = "   "
    with pytest.raises(ValueError, match="spec.storage.path must not be empty"):
        validate_experiment(valid_experiment)

def test_validate_experiment_missing_storage_path(valid_experiment):
    del valid_experiment["spec"]["storage"]["path"]
    with pytest.raises(ValueError, match="spec.storage.path must not be empty"):
        validate_experiment(valid_experiment)

def test_validate_experiment_invalid_scenario_ref(valid_experiment):
    valid_experiment["spec"]["scenarioRef"] = "not a dict"
    with pytest.raises(ValueError, match="spec.scenarioRef must be an object"):
        validate_experiment(valid_experiment)

def test_validate_experiment_missing_scenario_ref(valid_experiment):
    del valid_experiment["spec"]["scenarioRef"]
    with pytest.raises(ValueError, match="spec.scenarioRef must be an object"):
        validate_experiment(valid_experiment)

def test_validate_experiment_missing_scenario_source(valid_experiment):
    valid_experiment["spec"]["scenarioRef"] = {}
    with pytest.raises(ValueError, match="spec.scenarioRef must provide configMapName or inline scenario content"):
        validate_experiment(valid_experiment)

def test_validate_experiment_empty_scenario_source(valid_experiment):
    valid_experiment["spec"]["scenarioRef"] = {"configMapName": "", "inline": "   "}
    with pytest.raises(ValueError, match="spec.scenarioRef must provide configMapName or inline scenario content"):
        validate_experiment(valid_experiment)

def test_validate_experiment_inline_scenario(valid_experiment):
    valid_experiment["spec"]["scenarioRef"] = {"inline": "scenario content"}
    # Should not raise
    validate_experiment(valid_experiment)

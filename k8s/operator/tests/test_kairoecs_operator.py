from kairoecs_operator import render_status_patch


def test_render_status_patch_default_phase():
    experiment = {
        "apiVersion": "test.kairo.ecs/v1",
        "kind": "CustomExperiment",
        "metadata": {"name": "test-exp"},
    }
    result = render_status_patch(experiment)

    assert result == {
        "apiVersion": "test.kairo.ecs/v1",
        "kind": "CustomExperiment",
        "metadata": {"name": "test-exp"},
        "status": {
            "phase": "Rendered",
            "completedRuns": 0,
            "failedRuns": 0,
        },
    }


def test_render_status_patch_custom_phase():
    experiment = {
        "apiVersion": "test.kairo.ecs/v1",
        "kind": "CustomExperiment",
        "metadata": {"name": "test-exp"},
    }
    result = render_status_patch(experiment, phase="Running")

    assert result == {
        "apiVersion": "test.kairo.ecs/v1",
        "kind": "CustomExperiment",
        "metadata": {"name": "test-exp"},
        "status": {
            "phase": "Running",
            "completedRuns": 0,
            "failedRuns": 0,
        },
    }


def test_render_status_patch_missing_fields():
    experiment = {}
    result = render_status_patch(experiment)

    assert result == {
        "apiVersion": "kairo.ecs/v1alpha1",
        "kind": "KairoECSExperiment",
        "metadata": {"name": "kairo-experiment"},
        "status": {
            "phase": "Rendered",
            "completedRuns": 0,
            "failedRuns": 0,
        },
    }

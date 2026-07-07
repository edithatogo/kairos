import pytest
from kairoecs_operator import render_status_patch

def test_render_status_patch_default():
    experiment = {}
    patch = render_status_patch(experiment)

    assert patch == {
        "apiVersion": "kairo.ecs/v1alpha1",
        "kind": "KairoECSExperiment",
        "metadata": {"name": "kairo-experiment"},
        "status": {
            "phase": "Rendered",
            "completedRuns": 0,
            "failedRuns": 0,
        },
    }

def test_render_status_patch_custom_metadata():
    experiment = {
        "apiVersion": "custom/v1",
        "kind": "CustomExperiment",
        "metadata": {
            "name": "my-custom-experiment"
        }
    }
    patch = render_status_patch(experiment, phase="Running")

    assert patch == {
        "apiVersion": "custom/v1",
        "kind": "CustomExperiment",
        "metadata": {"name": "my-custom-experiment"},
        "status": {
            "phase": "Running",
            "completedRuns": 0,
            "failedRuns": 0,
        },
    }

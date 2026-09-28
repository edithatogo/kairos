import os
import subprocess
from pathlib import Path

import pytest
from kairoecs_operator import render_job, validate_experiment


def experiment_with_key(key):
    return {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "my-image:latest",
            "storage": {"backend": "s3", "path": "s3://bucket/out"},
            "scenarioRef": {"key": key, "inline": "scenario content"},
        },
    }


def test_inline_writer_keeps_metacharacters_in_filename_and_content_literal(tmp_path):
    scenario_content = "$(touch content-pwned); $HOME `id`"
    job = render_job(experiment_with_key("scenario.yaml"))
    writer = job["spec"]["template"]["spec"]["initContainers"][0]
    env = {item["name"]: item["value"] for item in writer["env"]}
    env.update(
        {
            "KAIRO_INLINE_SCENARIO": scenario_content,
            "KAIRO_SCENARIO_KEY": "scenario;$(touch key-pwned).yaml",
            "KAIRO_SCENARIO_DIR": str(tmp_path),
        }
    )

    subprocess.run(
        writer["command"],
        check=True,
        cwd=tmp_path,
        env={**os.environ, **env},
    )

    literal_filename = Path(env["KAIRO_SCENARIO_DIR"]) / env["KAIRO_SCENARIO_KEY"]
    assert literal_filename.read_text(encoding="utf-8") == scenario_content
    assert not (tmp_path / "key-pwned").exists()
    assert not (tmp_path / "content-pwned").exists()
    assert "scenario;$(touch" not in writer["command"][2]


@pytest.mark.parametrize(
    "key, message",
    [
        ("../../../etc/passwd", "path traversal components"),
        ("/etc/passwd", "absolute path"),
        ("C:\\Windows\\win.ini", "backslashes"),
        ("..\\..\\etc\\passwd", "backslashes"),
        ("%2e%2e%2fetc%2fpasswd", "URL-encoded paths"),
        ("%2e%2e%5cetc%5cpasswd", "URL-encoded paths"),
        ("scenario.yaml;$(touch-pwned)", "safe ConfigMap filename"),
    ],
)
def test_validate_experiment_rejects_unsafe_scenario_keys(key, message):
    with pytest.raises(ValueError, match=message):
        validate_experiment(experiment_with_key(key))


def test_validate_experiment_rejects_shell_metacharacters_before_rendering(tmp_path):
    marker = tmp_path / "pwned"
    key = "scenario;$(touch-pwned)"

    with pytest.raises(ValueError, match="safe ConfigMap filename"):
        render_job(experiment_with_key(key))

    assert not marker.exists()


def test_validate_experiment_rejects_invalid_config_map_name():
    experiment = experiment_with_key("scenario.yaml")
    experiment["spec"]["scenarioRef"]["configMapName"] = "../untrusted"

    with pytest.raises(ValueError, match="DNS subdomain"):
        validate_experiment(experiment)

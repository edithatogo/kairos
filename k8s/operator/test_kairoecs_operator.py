import importlib.util
from pathlib import Path
import unittest


OPERATOR_PATH = Path(__file__).with_name("kairoecs_operator.py")
SPEC = importlib.util.spec_from_file_location("kairoecs_operator", OPERATOR_PATH)
operator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(operator)


def experiment(scenario_ref):
    return {
        "kind": "KairoECSExperiment",
        "metadata": {"name": "security-test"},
        "spec": {
            "image": "kairo-ecs-cli:test",
            "scenarioRef": scenario_ref,
            "storage": {"backend": "filesystem", "path": "/output"},
        },
    }


class ScenarioKeyTests(unittest.TestCase):
    def test_default_and_custom_keys_render_as_paths(self):
        default_job = operator.render_job(experiment({"configMapName": "scenario-config"}))
        default_spec = default_job["spec"]["template"]["spec"]
        self.assertEqual(default_spec["containers"][0]["args"][2], "/scenario/scenario.yaml")

        custom_job = operator.render_job(experiment({"inline": "scenario: ok", "key": "run-01.yaml"}))
        custom_spec = custom_job["spec"]["template"]["spec"]
        init = custom_spec["initContainers"][0]
        env = {item["name"]: item["value"] for item in init["env"]}
        self.assertEqual(custom_spec["containers"][0]["args"][2], "/scenario/run-01.yaml")
        self.assertEqual(env["KAIRO_SCENARIO_PATH"], "/scenario/run-01.yaml")
        self.assertEqual(init["command"][2], 'printf \'%s\' "$KAIRO_INLINE_SCENARIO" > "$KAIRO_SCENARIO_PATH"')

    def test_unsafe_keys_are_rejected_for_both_scenario_sources(self):
        unsafe_keys = (
            "../outside.yaml",
            "/tmp/outside.yaml",
            r"..\outside.yaml",
            "$(touch /tmp/pwned)",
            "scenario.yaml; id",
            ".",
            "..",
            42,
        )
        for key in unsafe_keys:
            for scenario_ref in (
                {"configMapName": "scenario-config", "key": key},
                {"inline": "scenario: ok", "key": key},
            ):
                with self.subTest(key=key, scenario_ref=scenario_ref):
                    with self.assertRaisesRegex(ValueError, "safe filename"):
                        operator.render_job(experiment(scenario_ref))


class StatusPatchTests(unittest.TestCase):
    def test_default_phase_preserves_experiment_identity_and_initial_counts(self):
        experiment_resource = {
            "apiVersion": "test.kairo.ecs/v1",
            "kind": "CustomExperiment",
            "metadata": {"name": "test-exp"},
        }

        self.assertEqual(
            operator.render_status_patch(experiment_resource),
            {
                "apiVersion": "test.kairo.ecs/v1",
                "kind": "CustomExperiment",
                "metadata": {"name": "test-exp"},
                "status": {
                    "phase": "Rendered",
                    "completedRuns": 0,
                    "failedRuns": 0,
                },
            },
        )

    def test_custom_phase_is_preserved(self):
        experiment_resource = {
            "apiVersion": "test.kairo.ecs/v1",
            "kind": "CustomExperiment",
            "metadata": {"name": "test-exp"},
        }

        patch = operator.render_status_patch(experiment_resource, phase="Running")

        self.assertEqual(patch["status"]["phase"], "Running")
        self.assertEqual(patch["status"]["completedRuns"], 0)
        self.assertEqual(patch["status"]["failedRuns"], 0)

    def test_missing_identity_fields_use_documented_defaults(self):
        self.assertEqual(
            operator.render_status_patch({}),
            {
                "apiVersion": "kairo.ecs/v1alpha1",
                "kind": "KairoECSExperiment",
                "metadata": {"name": "kairo-experiment"},
                "status": {
                    "phase": "Rendered",
                    "completedRuns": 0,
                    "failedRuns": 0,
                },
            },
        )


if __name__ == "__main__":
    unittest.main()

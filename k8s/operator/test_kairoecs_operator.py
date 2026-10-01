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


def valid_experiment(scenario_ref=None):
    return {
        "kind": "KairoECSExperiment",
        "spec": {
            "image": "kairo-ecs-cli:test",
            "parallelism": 1,
            "scenarioRef": (
                scenario_ref
                if scenario_ref is not None
                else {"configMapName": "scenario-config"}
            ),
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


class ExperimentValidationTests(unittest.TestCase):
    def test_accepts_configmap_and_inline_scenarios(self):
        for scenario_ref in (
            {"configMapName": "scenario-config"},
            {"inline": "scenario: ok"},
        ):
            with self.subTest(scenario_ref=scenario_ref):
                operator.validate_experiment(valid_experiment(scenario_ref))

    def test_rejects_invalid_fields_with_specific_errors(self):
        cases = []

        invalid_kind = valid_experiment()
        invalid_kind["kind"] = "Deployment"
        cases.append((invalid_kind, "experiment kind must be KairoECSExperiment"))
        cases.append(({}, "experiment kind must be KairoECSExperiment"))
        cases.append(({"kind": "KairoECSExperiment"}, "experiment spec must be an object"))

        for spec in (None, "not a dict"):
            invalid = valid_experiment()
            invalid["spec"] = spec
            cases.append((invalid, "experiment spec must be an object"))

        for image in ("", "   "):
            invalid = valid_experiment()
            invalid["spec"]["image"] = image
            cases.append((invalid, "spec.image must not be empty"))
        invalid = valid_experiment()
        del invalid["spec"]["image"]
        cases.append((invalid, "spec.image must not be empty"))

        for parallelism in (0, -1):
            invalid = valid_experiment()
            invalid["spec"]["parallelism"] = parallelism
            cases.append((invalid, "spec.parallelism must be greater than zero"))

        invalid = valid_experiment()
        invalid["spec"]["storage"] = None
        cases.append((invalid, "spec.storage must be an object"))
        invalid = valid_experiment()
        del invalid["spec"]["storage"]
        cases.append((invalid, "spec.storage must be an object"))
        invalid = valid_experiment()
        invalid["spec"]["storage"]["backend"] = "unknown"
        cases.append((invalid, "spec.storage.backend must be one of azure, filesystem, gcs, s3"))

        for path in ("", "   "):
            invalid = valid_experiment()
            invalid["spec"]["storage"]["path"] = path
            cases.append((invalid, "spec.storage.path must not be empty"))
        invalid = valid_experiment()
        del invalid["spec"]["storage"]["path"]
        cases.append((invalid, "spec.storage.path must not be empty"))

        invalid = valid_experiment()
        invalid["spec"]["scenarioRef"] = None
        cases.append((invalid, "spec.scenarioRef must be an object"))
        invalid = valid_experiment()
        del invalid["spec"]["scenarioRef"]
        cases.append((invalid, "spec.scenarioRef must be an object"))
        invalid = valid_experiment()
        invalid["spec"]["scenarioRef"] = "not a dict"
        cases.append((invalid, "spec.scenarioRef must be an object"))

        for scenario_ref in ({}, {"configMapName": "", "inline": "   "}, {"inline": "   "}):
            invalid = valid_experiment(scenario_ref)
            cases.append(
                (
                    invalid,
                    "spec.scenarioRef must provide configMapName or inline scenario content",
                )
            )

        for invalid, message in cases:
            with self.subTest(message=message, invalid=invalid):
                with self.assertRaisesRegex(ValueError, message):
                    operator.validate_experiment(invalid)


class RenderJobTests(unittest.TestCase):
    def test_renders_configmap_scenario_and_storage_settings(self):
        experiment = valid_experiment(
            {"configMapName": "scenario-config", "key": "custom-scenario.yaml"}
        )
        experiment["metadata"] = {"name": "test-exp"}
        experiment["spec"].update(
            {
                "image": "kairo:latest",
                "parallelism": 2,
                "storage": {"backend": "s3", "path": "s3://bucket/test"},
                "resources": {"requests": {"cpu": "1", "memory": "1Gi"}},
                "checkpoint": {"enabled": False},
            }
        )

        job = operator.render_job(experiment)
        job_spec = job["spec"]
        template_spec = job_spec["template"]["spec"]
        container = template_spec["containers"][0]
        env = {entry["name"]: entry["value"] for entry in container["env"]}

        self.assertEqual(job["apiVersion"], "batch/v1")
        self.assertEqual(job["kind"], "Job")
        self.assertEqual(job["metadata"]["name"], "test-exp-run")
        self.assertEqual(job_spec["parallelism"], 2)
        self.assertEqual(job_spec["completions"], 2)
        self.assertEqual(template_spec["restartPolicy"], "Never")
        self.assertEqual(container["image"], "kairo:latest")
        self.assertEqual(container["resources"], {"requests": {"cpu": "1", "memory": "1Gi"}})
        self.assertEqual(env["KAIRO_STORAGE_BACKEND"], "s3")
        self.assertEqual(env["KAIRO_OUTPUT_URI"], "s3://bucket/test")
        self.assertEqual(env["KAIRO_CHECKPOINT_ENABLED"], "false")
        self.assertIn("/scenario/custom-scenario.yaml", container["args"])
        self.assertEqual(
            template_spec["volumes"][0]["configMap"]["name"], "scenario-config"
        )
        self.assertEqual(
            template_spec["volumes"][0]["configMap"]["items"],
            [{"key": "custom-scenario.yaml", "path": "custom-scenario.yaml"}],
        )

    def test_renders_inline_scenario_with_defaults(self):
        experiment = valid_experiment({"inline": "scenario-content-here"})

        job = operator.render_job(experiment)
        template_spec = job["spec"]["template"]["spec"]
        init_container = template_spec["initContainers"][0]
        init_env = {entry["name"]: entry["value"] for entry in init_container["env"]}
        container = template_spec["containers"][0]
        container_env = {entry["name"]: entry["value"] for entry in container["env"]}

        self.assertEqual(init_container["name"], "write-inline-scenario")
        self.assertEqual(init_env["KAIRO_INLINE_SCENARIO"], "scenario-content-here")
        self.assertEqual(init_env["KAIRO_SCENARIO_PATH"], "/scenario/scenario.yaml")
        self.assertEqual(
            init_container["command"][2],
            'printf \'%s\' "$KAIRO_INLINE_SCENARIO" > "$KAIRO_SCENARIO_PATH"',
        )
        self.assertEqual(job["spec"]["parallelism"], 1)
        self.assertEqual(job["spec"]["completions"], 1)
        self.assertEqual(container_env["KAIRO_CHECKPOINT_ENABLED"], "true")


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

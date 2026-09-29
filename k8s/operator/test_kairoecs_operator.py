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


if __name__ == "__main__":
    unittest.main()

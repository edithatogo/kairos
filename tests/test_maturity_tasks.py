"""Negative controls for task scope, dependency and executable-target integrity."""
from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("maturity_tasks", ROOT / "scripts/maturity_tasks.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Cannot locate maturity_tasks module at {ROOT / 'scripts/maturity_tasks.py'}")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MaturityTasksTests(unittest.TestCase):
    def setUp(self):
        self.data = json.loads((ROOT / MODULE.CATALOG).read_text())

    def reject(self, fragment):
        with self.assertRaisesRegex(ValueError, fragment):
            MODULE.validate(self.data, ROOT)

    def test_real_catalog(self):
        self.assertGreater(len(MODULE.validate(self.data, ROOT)), 17)

    def test_duplicate_id(self):
        self.data["tasks"].append(deepcopy(self.data["tasks"][0]))
        self.reject("Duplicate task")

    def test_cycle(self):
        first = self.data["tasks"][0]
        first["requires"] = [first["id"]]
        self.reject("Dependency cycle")

    def test_missing_dependency(self):
        self.data["tasks"][0]["requires"] = ["K99.missing"]
        self.reject("Unknown dependency")

    def test_path_escape(self):
        self.data["tasks"][0]["write"] = ["../outside.rs"]
        self.reject("escapes repository")

    def test_directory_scope_rejected(self):
        self.data["tasks"][0]["write"] = ["crates"]
        self.reject("not a file")

    def test_oversized_scope(self):
        self.data["tasks"][0]["write"] = [f"future-{i}.rs" for i in range(6)]
        self.reject("1..5 files")

    def test_unknown_cargo_package(self):
        self.data["tasks"][0]["checks"] = [{"cwd": ".", "argv": ["cargo", "test", "-p", "invented"]}]
        self.reject("unknown Cargo package")

    def test_missing_cargo_target(self):
        self.data["tasks"][0]["checks"] = [{"cwd": ".", "argv": ["cargo", "test", "-p", "kairo-ecs-core", "--test", "unowned_future_test"]}]
        self.reject("missing target must be an explicit output")

    def test_no_duplicate_status_store(self):
        self.data["tasks"][0]["status"] = "complete"
        self.reject("status belongs")

    def test_benchmark_requires_enabled_feature(self):
        self.data["tasks"][0]["checks"] = [{"cwd": ".", "argv": ["cargo", "bench", "-p", "kairo-ecs-pdes", "--bench", "production"]}]
        self.reject("requires features")

    def test_gated_test_cannot_silently_run_zero_tests(self):
        self.data["tasks"][0]["checks"] = [{"cwd": ".", "argv": ["cargo", "test", "-p", "kairo-ecs-pdes", "--test", "track48_rollback_regressions"]}]
        self.reject("requires features")

    def test_all_scientific_obligations_retained(self):
        self.data["tasks"] = [t for t in self.data["tasks"] if t.get("obligation") != "S17"]
        self.reject("coverage mismatch")

    def test_design_not_assigned_as_settled_local_work(self):
        task = next(t for t in self.data["tasks"] if t["kind"] == "design")
        task["executor"] = "local"
        self.reject("unresolved design needs a specialist")


if __name__ == "__main__":
    unittest.main()

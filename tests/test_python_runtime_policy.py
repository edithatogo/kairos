"""Keep active runtime selection distinct from historical provenance."""
import re
import sys
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class PythonRuntimePolicyTests(unittest.TestCase):
    def test_actual_interpreter_and_repo_selection_are_qualified_patch(self):
        self.assertEqual(sys.version_info[:3], (3, 15, 1))
        self.assertEqual((ROOT / ".python-version").read_text().strip(), "3.15.1")
        self.assertEqual(tomllib.loads((ROOT / "mise.toml").read_text())["tools"]["python"], "3.15.1")

    def test_active_ci_does_not_restore_older_python_runners(self):
        for path in (ROOT / ".github/workflows").glob("*.yml"):
            for line in path.read_text().splitlines():
                if "python-version:" in line:
                    for version in re.findall(r"3\.\d+(?:\.\d+)?", line):
                        self.assertEqual(version, "3.15.1", f"{path.name}: {line}")
        for name in ["bindings/python/pyproject.toml", "python/kairo_gym/pyproject.toml", "templates/bindings/python/pyproject.toml"]:
            self.assertEqual(tomllib.loads((ROOT / name).read_text())["project"]["requires-python"], ">=3.15.1,<3.16")

    def test_syft_execution_rejects_old_runtime(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location("python_policy_syft", ROOT / "scripts/supply_chain/install_verified_syft.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.require_supported_python()
        with self.assertRaisesRegex(RuntimeError, "Python 3.15.1 is required"):
            module.require_supported_python((3, 14, 8))


if __name__ == "__main__":
    unittest.main()

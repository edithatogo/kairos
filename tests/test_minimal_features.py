import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class MinimalFeatureTest(unittest.TestCase):
    def test_rejects_direct_domain_and_transitive_build_dependencies(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifests = {
                "consumer": (
                    '[dependencies]\ncareops_core = { path = "../careops_core" }\n'
                    '[build-dependencies]\nbuilder = { path = "../builder" }\n'
                ),
                "builder": (
                    '[dependencies]\npolars = { path = "../polars" }\n'
                    'bindgen = { path = "../bindgen" }\n'
                ),
                "bindgen": "",
                "careops_core": "",
                "polars": "",
            }
            for name, dependencies in manifests.items():
                crate = root / name
                (crate / "src").mkdir(parents=True)
                (crate / "src/lib.rs").touch()
                (crate / "Cargo.toml").write_text(
                    f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n\n{dependencies}',
                    encoding="utf-8",
                )
            manifest = root / "consumer/Cargo.toml"
            subprocess.run(
                [
                    "cargo",
                    "generate-lockfile",
                    "--offline",
                    "--manifest-path",
                    str(manifest),
                ],
                cwd=ROOT,
                check=True,
                capture_output=True,
            )
            result = subprocess.run(
                [
                    sys.executable,
                    ROOT / "scripts/check_minimal_features.py",
                    "--manifest-path",
                    manifest,
                ],
                cwd=ROOT,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn(
                "Optional dependencies in the minimal Rust profile: bindgen, careops-core, polars",
                result.stderr,
            )

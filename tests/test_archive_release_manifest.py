import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
import test_package_archive_bundle as fixtures

spec = importlib.util.spec_from_file_location("archive_manifest", Path(__file__).resolve().parents[1] / "packaging/scripts/build_archive_release_manifest.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class ArchiveManifestTests(unittest.TestCase):
    def test_actual_archive_subjects_and_wrong_source_fail_closed(self):
        original = fixtures.MODULE.verify
        calls = []
        def inspect(source):
            original(source)
            if calls:
                return
            calls.append(True)
            output = source.parent / "release"
            with self.assertRaises(ValueError):
                module.build(source, output, "2" * 40)
            self.assertFalse(output.exists())
            manifest = module.build(source, output, "1" * 40)
            self.assertEqual({r["ecosystem"] for r in manifest["artifacts"]}, {"rust", "python", "r", "julia", "typescript", "csharp", "go"})
            self.assertFalse(manifest["production_publish_enabled"])
            for row in manifest["artifacts"]:
                self.assertEqual(fixtures.MODULE.checksum(output / row["path"]), row["sha256"])
                self.assertEqual((output / row["path"]).read_bytes(), (source / row["path"].removeprefix("archives/")).read_bytes())
            with self.assertRaises(ValueError):
                module.build(source, output, "1" * 40)
            checksum_bytes = (source / "SHA256SUMS").read_bytes()
            (source / "SHA256SUMS").unlink()
            with self.assertRaises((ValueError, FileNotFoundError)):
                module.build(source, source.parent / "missing-evidence", "1" * 40)
            (source / "SHA256SUMS").write_bytes(checksum_bytes)
        with patch.object(fixtures.MODULE, "verify", side_effect=inspect):
            fixtures.PackageArchiveBundleTests().test_build_indexes_and_verifies_all_seven_ecosystems()
        self.assertEqual(len(calls), 1)

if __name__ == "__main__":
    unittest.main()

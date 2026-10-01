import importlib.util
import json
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "packaging/scripts/build_package_archive_bundle.py"
SPEC = importlib.util.spec_from_file_location("package_archive_bundle", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PackageArchiveBundleTests(unittest.TestCase):
    def test_build_indexes_and_verifies_all_seven_ecosystems(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "packages"
            output = root / "retained"
            for ecosystem, (_, extensions) in MODULE.ARCHIVES.items():
                directory = source / ecosystem
                directory.mkdir(parents=True)
                (directory / "BUILD-INFO.json").write_text(
                    json.dumps({
                        "ecosystem": ecosystem,
                        "source_commit": "1" * 40,
                        "command": "build sample archive",
                        "toolchain": "unit test",
                        "platform": "test platform",
                        "exit_status": 0,
                        "validation_notes": "",
                    }),
                    encoding="utf-8",
                )
                suffix = next(iter(extensions))
                archive_path = directory / f"sample{suffix}"
                if suffix in {".whl", ".nupkg"}:
                    with zipfile.ZipFile(archive_path, "w") as archive:
                        archive.writestr("package/file.txt", "contents")
                else:
                    payload = root / "payload.txt"
                    payload.write_text("contents", encoding="utf-8")
                    with tarfile.open(archive_path, "w:gz") as archive:
                        archive.add(payload, arcname="package/file.txt")

            MODULE.build(source, output, "1" * 40)
            MODULE.verify(output)
            self.assertEqual(
                {
                    row["ecosystem"]
                    for row in json.loads(
                        (output / "ARCHIVE-INDEX.json").read_text(encoding="utf-8")
                    )["artifacts"]
                },
                set(MODULE.ARCHIVES),
            )

            archive = next(
                path
                for path in output.rglob("*")
                if path.is_file() and path.name.endswith(".crate")
            )
            archive.write_bytes(b"tampered")
            with self.assertRaises(ValueError):
                MODULE.verify(output)


if __name__ == "__main__":
    unittest.main()

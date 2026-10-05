import importlib.util
import json
import tarfile
import tempfile
import unittest
import zipfile
from unittest import mock
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "packaging/scripts/build_package_archive_bundle.py"
SPEC = importlib.util.spec_from_file_location("package_archive_bundle", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def sample_bundle(root: Path, source_commit: str = "1" * 40) -> Path:
    root.mkdir(parents=True, exist_ok=True)
    source = root / "packages"
    output = root / "retained"
    payload = root / "payload.txt"
    payload.write_text("contents", encoding="utf-8")
    for ecosystem, (_, extensions) in MODULE.ARCHIVES.items():
        directory = source / ecosystem
        directory.mkdir(parents=True)
        (directory / "BUILD-INFO.json").write_text(
            json.dumps({
                "ecosystem": ecosystem,
                "source_commit": source_commit,
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
            with tarfile.open(archive_path, "w:gz") as archive:
                archive.add(payload, arcname="package/file.txt")
    MODULE.build(source, output, source_commit)
    return output


def file_hashes(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): MODULE.checksum(path)
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


class PackageArchiveBundleTests(unittest.TestCase):
    def test_indexed_paths_fail_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for path in ["../outside.crate", "/tmp/archive.crate", "x/../y.crate", "x//y.crate", "C:/archive.crate", "x\\y.crate", "", None]:
                with self.subTest(path=path), self.assertRaises(ValueError):
                    MODULE.indexed_path(root, path)
            external = root.parent / (root.name + "-external")
            external.mkdir()
            try:
                (root / "linked").symlink_to(external, target_is_directory=True)
                with self.assertRaises(ValueError):
                    MODULE.indexed_path(root, "linked/archive.crate")
                (root / "inside").mkdir()
                (root / "alias").symlink_to(root / "inside", target_is_directory=True)
                with self.assertRaises(ValueError):
                    MODULE.indexed_path(root, "alias/archive.crate")
                self.assertEqual(MODULE.indexed_path(root, "rust/package.crate"), root / "rust/package.crate")
            finally:
                external.rmdir()

    def test_tgz_must_be_a_gzip_tar_archive(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "package.tgz"
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("package/file.txt", "contents")
            with self.assertRaises(tarfile.ReadError):
                MODULE.validate_archive(path)

    def test_build_indexes_and_verifies_all_seven_ecosystems(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = sample_bundle(root)
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

    def test_verify_existing_is_read_only_and_source_bound(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = sample_bundle(Path(temporary))
            before = file_hashes(output)
            args = [
                "build_package_archive_bundle.py",
                "--verify-existing",
                "--output", str(output),
                "--source-commit", "1" * 40,
            ]
            with mock.patch("sys.argv", args):
                MODULE.main()
            self.assertEqual(file_hashes(output), before)

            args[-1] = "2" * 40
            with mock.patch("sys.argv", args), self.assertRaisesRegex(
                ValueError, "differs from the expected acquisition commit"
            ):
                MODULE.main()
            self.assertEqual(file_hashes(output), before)

    def test_verify_existing_rejects_bad_source_and_input_modes(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = sample_bundle(Path(temporary))
            base = ["build_package_archive_bundle.py", "--output", str(output)]
            for args in (
                base + ["--verify-existing"],
                base + ["--verify-existing", "--source-commit", "1" * 39],
                base + ["--verify-existing", "--source-commit", "1" * 40, "--input", "unused"],
                ["build_package_archive_bundle.py", "--output", str(output), "--source-commit", "1" * 40],
            ):
                with self.subTest(args=args), mock.patch("sys.argv", args), self.assertRaises(SystemExit) as raised:
                    MODULE.main()
                self.assertEqual(raised.exception.code, 2)

    def test_verify_existing_rejects_missing_index_and_archive(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = sample_bundle(Path(temporary))
            index = output / "ARCHIVE-INDEX.json"
            index_bytes = index.read_bytes()
            index.unlink()
            with self.assertRaisesRegex(ValueError, "metadata is missing"):
                MODULE.verify(output, expected_source_commit="1" * 40)
            index.write_bytes(index_bytes)

            archive = next(path for path in output.rglob("*") if path.is_file() and path.name.endswith(".crate"))
            archive.unlink()
            with self.assertRaisesRegex(ValueError, "missing or is a symlink"):
                MODULE.verify(output, expected_source_commit="1" * 40)

    def test_verify_existing_rejects_symlinked_metadata(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = sample_bundle(root / "case")
            alias = root / "bundle-alias"
            alias.symlink_to(output, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "metadata must not be a symlink"):
                MODULE.verify(alias, expected_source_commit="1" * 40)
            for name in ("ARCHIVE-INDEX.json", "BUILD-RECEIPT.json", "SHA256SUMS"):
                with self.subTest(name=name):
                    path = output / name
                    data = path.read_bytes()
                    external = root / f"external-{name}"
                    external.write_bytes(data)
                    path.unlink()
                    path.symlink_to(external)
                    with self.assertRaisesRegex(ValueError, "metadata must not be a symlink"):
                        MODULE.verify(output, expected_source_commit="1" * 40)
                    path.unlink()
                    path.write_bytes(data)

    def test_verify_existing_rejects_checksum_inventory_drift(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = sample_bundle(Path(temporary))
            sums = output / "SHA256SUMS"
            sums.write_text("0" * 64 + "  rust/sample.crate\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "SHA256SUMS does not match"):
                MODULE.verify(output, expected_source_commit="1" * 40)

    def test_verify_rejects_missing_or_malformed_index_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = sample_bundle(Path(temporary))
            index_path = output / "ARCHIVE-INDEX.json"
            index = json.loads(index_path.read_text(encoding="utf-8"))
            index.pop("source_commit")
            index_path.write_text(json.dumps(index), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "no full source commit"):
                MODULE.verify(output)
            index["source_commit"] = "G" * 40
            index_path.write_text(json.dumps(index), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "no full source commit"):
                MODULE.verify(output)

    def test_verify_rejects_missing_or_mismatched_ecosystem_source_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = sample_bundle(Path(temporary))
            receipt_path = output / "BUILD-RECEIPT.json"
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            receipt["ecosystems"]["rust"]["source_commit"] = "2" * 40
            receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "does not match source"):
                MODULE.verify(output, expected_source_commit="1" * 40)

            receipt["ecosystems"].pop("rust")
            receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "cover all seven ecosystems"):
                MODULE.verify(output, expected_source_commit="1" * 40)

    def test_verify_rejects_nontext_or_blank_receipt_metadata(self):
        invalid_values = ({}, [], 17, " \t")
        for field in ("command", "toolchain", "platform"):
            for invalid in invalid_values:
                with self.subTest(field=field, invalid=invalid), tempfile.TemporaryDirectory() as temporary:
                    output = sample_bundle(Path(temporary))
                    receipt_path = output / "BUILD-RECEIPT.json"
                    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
                    receipt["ecosystems"]["rust"][field] = invalid
                    receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, "does not match source or successful build"):
                        MODULE.verify(output, expected_source_commit="1" * 40)

    def test_build_rejects_nontext_or_blank_receipt_metadata(self):
        invalid_values = ({}, [], 17, " \t")
        for field in ("command", "toolchain", "platform"):
            for invalid in invalid_values:
                with self.subTest(field=field, invalid=invalid), tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    source = root / "packages"
                    ecosystem = "rust"
                    directory = source / ecosystem
                    directory.mkdir(parents=True)
                    (directory / "BUILD-INFO.json").write_text(json.dumps({
                        "ecosystem": ecosystem,
                        "source_commit": "1" * 40,
                        "command": "build sample archive",
                        "toolchain": "unit test",
                        "platform": "test platform",
                        "exit_status": 0,
                        field: invalid,
                    }), encoding="utf-8")
                    archive_path = directory / "sample.crate"
                    with tarfile.open(archive_path, "w:gz") as archive:
                        archive.addfile(tarfile.TarInfo("package/file.txt"))
                    with self.assertRaisesRegex(ValueError, "build receipt is incomplete"):
                        MODULE.build(source, root / "retained", "1" * 40)

    def test_verify_existing_rejects_unsafe_indexed_archive_path(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = sample_bundle(Path(temporary))
            index_path = output / "ARCHIVE-INDEX.json"
            index = json.loads(index_path.read_text(encoding="utf-8"))
            index["artifacts"][0]["path"] = "../outside.crate"
            index_path.write_text(json.dumps(index), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "canonical|escapes"):
                MODULE.verify(output, expected_source_commit="1" * 40)


if __name__ == "__main__":
    unittest.main()

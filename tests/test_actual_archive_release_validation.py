from __future__ import annotations

import hashlib
import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "packaging/scripts"))
import validate_actual_archive_release as validation


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class ActualArchiveReleaseValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name).resolve()
        self.release = self.root / "release"
        self.index_path = self.root / "verified-archive-index.json"
        self.source_commit = "a" * 40
        self.rows = []
        entries = [
            ("go", "go-source-archive", "go/example.tar.gz"),
            ("julia", "julia-source-archive", "julia/example.tar.gz"),
            ("nuget", "nuget-package", "nuget/example.nupkg"),
            ("python", "python-distribution", "python/example-1.whl"),
            ("python", "python-distribution", "python/example-2.tar.gz"),
            ("r", "r-source-package", "r/example.tar.gz"),
            ("rust", "crate", "rust/example.crate"),
            ("typescript", "npm-package", "typescript/example.tgz"),
        ]
        for position, (ecosystem, kind, relative) in enumerate(entries):
            content = f"fixture archive bytes {position}\n".encode()
            self.rows.append({
                "ecosystem": ecosystem,
                "kind": kind,
                "path": relative,
                "bytes": len(content),
                "sha256": sha(content),
                "builder": {
                    "ecosystem": ecosystem,
                    "source_commit": self.source_commit,
                    "exit_status": 0,
                    "command": f"build {ecosystem} fixture",
                    "toolchain": "fixture-toolchain 1.0",
                    "platform": "test-platform",
                },
            })
        self.rows.sort(key=lambda row: row["path"])
        self.index = {
            "schema_version": 1,
            "source_commit": self.source_commit,
            "created_at_utc": "2026-10-06T00:00:00+00:00",
            "artifacts": self.rows,
        }
        self.index_bytes = json.dumps(self.index, sort_keys=True, indent=2).encode() + b"\n"
        self.index_path.write_bytes(self.index_bytes)
        self.index_sha256 = sha(self.index_bytes)
        self._write_release()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _write_release(self) -> None:
        shutil.rmtree(self.release, ignore_errors=True)
        self.release.mkdir()
        manifest, _, _ = validation._expected_outputs(self.rows, self.source_commit, self.index_sha256)
        for row in self.rows:
            content = f"fixture archive bytes {self.rows.index(row)}\n".encode()
            target = self.release / "archives" / row["path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(content)
        (self.release / "release-artifact-manifest.json").write_text(
            json.dumps(manifest, sort_keys=True, indent=2) + "\n", encoding="utf-8"
        )
        (self.release / "SHA256SUMS").write_text(
            "".join(f"{row['sha256']}  {row['path']}\n" for row in manifest["artifacts"]),
            encoding="utf-8",
        )
        (self.release / "RELEASE.txt").write_text(
            f"Verified package archives from {self.source_commit}. Evidence preparation only; publication disabled.\n",
            encoding="utf-8",
        )

    def _validate(self, *, source: str | None = None, index_path: Path | None = None,
                  index_sha: str | None = None, release: Path | None = None) -> dict[str, object]:
        return validation.validate_release(
            release or self.release,
            index_path or self.index_path,
            index_sha or self.index_sha256,
            source or self.source_commit,
        )

    def test_accepts_exact_eight_archive_actual_release_tree(self) -> None:
        report = self._validate()
        self.assertEqual(report["schema"], "kairos-actual-archive-release-readback-v1")
        self.assertEqual(report["result"], "pass")
        self.assertEqual(report["release_stage"], "actual-package-archives")
        self.assertEqual(report["archive_count"], 8)
        self.assertEqual(report["ecosystem_count"], 7)
        self.assertEqual(report["claim_scope"], validation.CLAIM_SCOPE)
        self.assertNotIn("sbom", report)
        self.assertNotIn("provenance", report)

    def test_rejects_wrong_source_or_archive_index_pin(self) -> None:
        with self.assertRaisesRegex(validation.ValidationError, "source_commit"):
            self._validate(source="b" * 40)
        with self.assertRaisesRegex(validation.ValidationError, "archive_index_sha256_mismatch"):
            self._validate(index_sha="0" * 64)
        with self.assertRaisesRegex(validation.ValidationError, "archive_index_sha256_invalid"):
            self._validate(index_sha="A" * 64)

    def test_rejects_missing_and_extra_output_files_or_directories(self) -> None:
        (self.release / "RELEASE.txt").unlink()
        with self.assertRaisesRegex(validation.ValidationError, "release_tree_inventory_mismatch"):
            self._validate()
        self._write_release()
        (self.release / "unlisted.txt").write_text("extra", encoding="utf-8")
        with self.assertRaisesRegex(validation.ValidationError, "release_tree_inventory_mismatch"):
            self._validate()
        self._write_release()
        (self.release / "archives/unused").mkdir()
        with self.assertRaisesRegex(validation.ValidationError, "release_tree_inventory_mismatch"):
            self._validate()

    def test_rejects_manifest_mutations_including_publish_true(self) -> None:
        path = self.release / "release-artifact-manifest.json"
        original = json.loads(path.read_text())
        for field, value in (("production_publish_enabled", True),
                             ("source_commit", "b" * 40),
                             ("archive_index_sha256", "0" * 64)):
            with self.subTest(field=field):
                changed = dict(original)
                changed[field] = value
                path.write_text(json.dumps(changed), encoding="utf-8")
                with self.assertRaisesRegex(validation.ValidationError, "release_manifest_mismatch"):
                    self._validate()
        changed = dict(original)
        changed["artifacts"] = changed["artifacts"][:-1]
        path.write_text(json.dumps(changed), encoding="utf-8")
        with self.assertRaisesRegex(validation.ValidationError, "release_manifest_mismatch"):
            self._validate()

    def test_manifest_schema_rejects_boolean_or_float_integer_fields(self) -> None:
        path = self.release / "release-artifact-manifest.json"
        original = json.loads(path.read_text())
        for mutate in (
            lambda value: value.__setitem__("schema_version", True),
            lambda value: value["artifacts"][0].__setitem__("bytes", True),
            lambda value: value["artifacts"][0].__setitem__("bytes", float(value["artifacts"][0]["bytes"])),
        ):
            changed = json.loads(json.dumps(original))
            mutate(changed)
            path.write_text(json.dumps(changed), encoding="utf-8")
            with self.assertRaisesRegex(validation.ValidationError, "release_manifest_mismatch"):
                self._validate()

    def test_rejects_changed_archive_copy_and_checksum_text(self) -> None:
        archive = self.release / "archives/rust/example.crate"
        archive.write_bytes(b"altered")
        with self.assertRaisesRegex(validation.ValidationError, "release_archive_mismatch"):
            self._validate()
        self._write_release()
        (self.release / "SHA256SUMS").write_text("0" * 64 + "  archives/rust/example.crate\n", encoding="utf-8")
        with self.assertRaisesRegex(validation.ValidationError, "release_checksums_mismatch"):
            self._validate()

    def test_rejects_changed_release_text(self) -> None:
        (self.release / "RELEASE.txt").write_text("publication enabled\n", encoding="utf-8")
        with self.assertRaisesRegex(validation.ValidationError, "release_text_mismatch"):
            self._validate()

    def test_rejects_output_root_and_entry_symlinks(self) -> None:
        alias = self.root / "release-alias"
        alias.symlink_to(self.release, target_is_directory=True)
        with self.assertRaises(OSError):
            self._validate(release=alias)
        archive = self.release / "archives/rust/example.crate"
        archive.unlink()
        archive.symlink_to(self.index_path)
        with self.assertRaisesRegex(validation.ValidationError, "release_tree_symlink"):
            self._validate()

    def test_rejects_symlinked_archive_index_and_parent(self) -> None:
        link = self.root / "index-link.json"
        link.symlink_to(self.index_path)
        with self.assertRaises(OSError):
            self._validate(index_path=link)
        parent = self.root / "index-parent-link"
        parent.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(OSError):
            self._validate(index_path=parent / self.index_path.name)

    def test_rejects_input_index_inside_release_tree(self) -> None:
        internal = self.release / "verified-index.json"
        internal.write_bytes(self.index_bytes)
        with self.assertRaisesRegex(validation.ValidationError, "archive_index_must_be_outside_release_root"):
            self._validate(index_path=internal)

    def test_rejects_index_schema_source_and_row_shape_drift(self) -> None:
        mutations = []
        bad = dict(self.index)
        bad["source_commit"] = "b" * 40
        mutations.append(bad)
        bad = dict(self.index)
        bad["unexpected"] = True
        mutations.append(bad)
        bad = dict(self.index)
        bad["artifacts"] = self.rows[:-1]
        mutations.append(bad)
        for item in mutations:
            raw = json.dumps(item).encode()
            self.index_path.write_bytes(raw)
            with self.subTest(index=item):
                with self.assertRaises(validation.ValidationError):
                    self._validate(index_sha=sha(raw))

    def test_rejects_noncanonical_aliased_or_duplicate_index_paths(self) -> None:
        for path_value in ("rust/../rust/example.crate", "rust//example.crate", "rust\\example.crate"):
            bad_rows = [dict(row) for row in self.rows]
            bad_rows[0]["path"] = path_value
            self._write_index(bad_rows)
            with self.subTest(path=path_value), self.assertRaises(validation.ValidationError):
                self._validate(index_sha=sha(self.index_path.read_bytes()))
        bad_rows = [dict(row) for row in self.rows]
        bad_rows[-1]["path"] = bad_rows[-2]["path"].upper()
        self._write_index(bad_rows)
        with self.assertRaises(validation.ValidationError):
            self._validate(index_sha=sha(self.index_path.read_bytes()))

    def test_rejects_archive_file_directory_prefix_collision(self) -> None:
        bad_rows = [dict(row) for row in self.rows]
        next(row for row in bad_rows if row["path"] == "python/example-2.tar.gz")["path"] = (
            "python/example-1.whl/nested.tar.gz"
        )
        bad_rows.sort(key=lambda row: row["path"])
        self._write_index(bad_rows)
        with self.assertRaisesRegex(validation.ValidationError, "archive_index_output_path_collision"):
            self._validate(index_sha=sha(self.index_path.read_bytes()))

    def test_rejects_parent_component_path_aliases(self) -> None:
        with self.assertRaisesRegex(validation.ValidationError, "release_root_path_parent_component"):
            self._validate(release=self.root / "release" / ".." / "release")
        with self.assertRaisesRegex(validation.ValidationError, "archive_index_path_parent_component"):
            self._validate(index_path=self.root / "subdir" / ".." / self.index_path.name)

    def test_rejects_filesystem_root_as_release_output(self) -> None:
        with self.assertRaisesRegex(validation.ValidationError, "release_root_path_invalid"):
            self._validate(release=Path("/"))

    def test_rejects_bad_ecosystem_kind_digest_size_and_builder(self) -> None:
        for field, value in (("ecosystem", "unknown"), ("kind", "wrong-kind"),
                             ("sha256", "A" * 64), ("bytes", True),
                             ("builder", {"exit_status": False})):
            bad_rows = [dict(row) for row in self.rows]
            bad_rows[0][field] = value
            self._write_index(bad_rows)
            with self.subTest(field=field), self.assertRaises(validation.ValidationError):
                self._validate(index_sha=sha(self.index_path.read_bytes()))

    def test_rejects_missing_ecosystem_coverage_and_duplicate_case_paths(self) -> None:
        bad_rows = [dict(row) for row in self.rows]
        bad_rows[0]["ecosystem"] = "python"
        self._write_index(bad_rows)
        with self.assertRaises(validation.ValidationError):
            self._validate(index_sha=sha(self.index_path.read_bytes()))

    def test_rejects_duplicate_and_nonfinite_json(self) -> None:
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":1e999}'):
            with self.subTest(raw=raw):
                self.index_path.write_bytes(raw)
                with self.assertRaises(validation.ValidationError):
                    self._validate(index_sha=sha(raw))

    def test_cli_reports_failure_without_leaking_exceptions(self) -> None:
        from contextlib import redirect_stdout
        import io
        output = io.StringIO()
        with redirect_stdout(output):
            code = validation.main([
                "--release-root", str(self.release),
                "--verified-archive-index", str(self.index_path),
                "--archive-index-sha256", "0" * 64,
                "--release-source-commit", self.source_commit,
            ])
        self.assertEqual(code, 1)
        report = json.loads(output.getvalue())
        self.assertEqual(report["schema"], "kairos-actual-archive-release-readback-v1")
        self.assertEqual(report["error"], "archive_index_sha256_mismatch")

    def _write_index(self, rows: list[dict[str, object]]) -> None:
        value = dict(self.index)
        value["artifacts"] = rows
        self.index_path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")


if __name__ == "__main__":
    unittest.main()

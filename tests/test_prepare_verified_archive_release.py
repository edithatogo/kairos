from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "packaging/scripts/prepare_verified_archive_release.py"
SPEC = importlib.util.spec_from_file_location("prepare_verified_archive_release", SCRIPT)
gate = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(gate)


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class ArchiveReleaseAdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="archive-release-adapter-")
        self.root = Path(self.temp.name).resolve()
        self.source = "1" * 40
        self.acquisition = self.root / "acquisition"
        self.bundle = self.acquisition / "bundle"
        self.evidence = self.root / "evidence"
        self.external = self.root / "independent"
        self.output_parent = self.root / "output"
        for directory in (self.bundle, self.evidence, self.external, self.output_parent):
            directory.mkdir(parents=True, exist_ok=True)
        archive_rows = (
            ("rust/kairo-test-1.0.0.crate", "rust", "crate"),
            ("rust/kairo-extra-1.0.0.crate", "rust", "crate"),
            ("python/kairo_test-1.0.0-py3-none-any.whl", "python", "python-distribution"),
            ("r/kairo_test_1.0.0.tar.gz", "r", "r-source-package"),
            ("julia/kairo-test-1.0.0.tar.gz", "julia", "julia-source-archive"),
            ("typescript/kairo-test-1.0.0.tgz", "typescript", "npm-package"),
            ("nuget/Kairo.Test.1.0.0.nupkg", "nuget", "nuget-package"),
            ("go/kairo-test-1.0.0.tar.gz", "go", "go-source-archive"),
        )
        self.archive_bytes_by_path: dict[str, bytes] = {}
        self.rows = []
        self.builder_receipts = {
            ecosystem: {"ecosystem": ecosystem, "source_commit": self.source, "exit_status": 0,
                       "command": "synthetic archive fixture", "toolchain": "fixture", "platform": "fixture"}
            for ecosystem in gate.ECOSYSTEMS
        }
        for position, (path, ecosystem, kind) in enumerate(archive_rows):
            data = f"qualified archive fixture {position}".encode()
            self.archive_bytes_by_path[path] = data
            self.rows.append({"path": path, "sha256": sha(data), "bytes": len(data),
                              "ecosystem": ecosystem, "kind": kind,
                              "builder": self.builder_receipts[ecosystem]})
            archive_path = self.bundle / path
            archive_path.parent.mkdir(parents=True, exist_ok=True)
            archive_path.write_bytes(data)
        self.rows.sort(key=lambda row: row["path"])
        self.row = self.rows[0]
        index = {"schema_version": 1, "source_commit": self.source,
                 "created_at_utc": "2026-10-06T00:00:00Z", "artifacts": self.rows}
        self.index_bytes = json.dumps(index, separators=(",", ":")).encode()
        (self.bundle / "ARCHIVE-INDEX.json").write_bytes(self.index_bytes)
        self.receipt_bytes = json.dumps({"source_commit": self.source,
                                         "ecosystems": self.builder_receipts}, separators=(",", ":")).encode()
        (self.bundle / "BUILD-RECEIPT.json").write_bytes(self.receipt_bytes)
        self.archive_zip = self.acquisition / "artifact.zip"
        self.archive_zip.write_bytes(b"ZIP evidence fixture")
        self.schema_bytes = b'{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object"}'
        self.schema = self.external / "spdx.schema.json"
        self.schema.write_bytes(self.schema_bytes)
        self.binding = self.external / "expected-binding.json"
        self.binding.write_text(json.dumps({"source_commit": self.source,
                                           "spdx_schema_sha256": sha(self.schema_bytes)}))
        dependencies = [
            {"id": "packaging/scripts/" + name,
             "sha256": sha((SCRIPT.parent / name).read_bytes())}
            for name in gate.HELPERS
        ]
        dependencies.append({"id": "schema:spdx-2.3", "sha256": sha(self.schema_bytes)})
        self.inputs = self.external / "expected-inputs.json"
        self.inputs.write_text(json.dumps({"source_commit": self.source,
                                           "archive_index_sha256": sha(self.index_bytes),
                                           "dependencies": dependencies}))
        self.output = self.output_parent / "actual-archives"
        self.args = gate.argparse.Namespace(
            evidence_dir=self.evidence,
            archive_bundle=self.bundle,
            archive_zip=self.archive_zip,
            acquisition_dir=self.acquisition,
            expected_inputs=self.inputs,
            expected_binding=self.binding,
            expected_verifier_sha256=sha(SCRIPT.parent.joinpath(gate.VERIFIER).read_bytes()),
            spdx_schema=self.schema,
            release_source_commit=self.source,
            output=self.output,
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _verified_stdout(self) -> bytes:
        value = {
            "valid": True,
            "profile": gate.PROFILE,
            "archive_count": len(self.rows),
            "ecosystem_count": 7,
            "spdx_document_count": len(self.rows) + 1,
            "evidence_file_count": 10,
            "archive_index_sha256": sha(self.index_bytes),
            "statement_sha256": "a" * 64,
            "claim_scope": gate.CLAIM_SCOPE,
        }
        return json.dumps(value, separators=(",", ":")).encode()

    def _write_builder_output(self, output: Path, *, extra: bool = False) -> None:
        output.mkdir()
        artifacts = []
        for row in self.rows:
            relative = "archives/" + row["path"]
            artifacts.append({"path": relative, "sha256": row["sha256"], "bytes": row["bytes"],
                              "ecosystem": "csharp" if row["ecosystem"] == "nuget" else row["ecosystem"],
                              "archive_ecosystem": row["ecosystem"], "kind": row["kind"]})
            archive_path = output / relative
            archive_path.parent.mkdir(parents=True, exist_ok=True)
            archive_path.write_bytes(self.archive_bytes_by_path[row["path"]])
        artifacts.sort(key=lambda item: item["path"])
        manifest = {"schema_version": 1, "release_stage": "actual-package-archives",
                    "source_commit": self.source, "production_publish_enabled": False,
                    "archive_index_sha256": sha(self.index_bytes), "artifacts": artifacts}
        (output / "release-artifact-manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
        (output / "SHA256SUMS").write_text("".join(
            f"{item['sha256']}  {item['path']}\n" for item in sorted(artifacts, key=lambda item: item["path"])))
        (output / "RELEASE.txt").write_text(
            f"Verified package archives from {self.source}. Evidence preparation only; publication disabled.\n")
        if extra:
            (output / "unexpected.txt").write_text("extra")

    def _runner(self, *, verifier_code: int = 0, extra: bool = False):
        calls: list[list[str]] = []

        def run(argv: list[str], runner=None):
            calls.append(argv)
            if argv[1].endswith(gate.VERIFIER):
                return verifier_code, self._verified_stdout(), b"bounded stderr"
            out = Path(argv[argv.index("--output") + 1])
            self._write_builder_output(out, extra=extra)
            return 0, b"generated\n", b""

        return calls, run

    def test_verifier_result_requires_exact_profile_and_claim_scope(self) -> None:
        gate.validate_verifier_result(self._verified_stdout(), len(self.rows), sha(self.index_bytes))
        data = json.loads(self._verified_stdout())
        data["claim_scope"] = "release accepted"
        with self.assertRaises(gate.GateError):
            gate.validate_verifier_result(json.dumps(data).encode(), 1, sha(self.index_bytes))
        data = json.loads(self._verified_stdout())
        data["profile"] = "other"
        with self.assertRaises(gate.GateError):
            gate.validate_verifier_result(json.dumps(data).encode(), 1, sha(self.index_bytes))

    def test_strict_json_rejects_duplicate_nonfinite_and_overflow_numbers(self) -> None:
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":1e999}'):
            with self.subTest(raw=raw), self.assertRaises(gate.GateError):
                gate.strict_json(raw, "test")

    def test_symlink_paths_reject(self) -> None:
        target = self.root / "real-dir"
        target.mkdir()
        link = self.root / "directory-link"
        link.symlink_to(target, target_is_directory=True)
        with self.assertRaises(gate.GateError):
            gate.inspect_path(link, kind="directory")

    def test_prepare_runs_verifier_before_builder_and_validates_exact_output(self) -> None:
        calls, runner = self._runner()
        with mock.patch.object(gate, "run_bounded", side_effect=runner):
            result = gate.prepare(self.args)
        self.assertTrue(result["valid"])
        self.assertEqual(result["archive_count"], 8)
        self.assertTrue(calls[0][1].endswith(gate.VERIFIER))
        self.assertTrue(calls[1][1].endswith("build_archive_release_manifest.py"))
        self.assertEqual(calls[0][0], sys.executable)
        self.assertNotEqual(calls[0][calls[0].index("--expected-inputs") + 1], str(self.inputs))
        self.assertNotEqual(calls[0][calls[0].index("--spdx-schema") + 1], str(self.schema))
        self.assertEqual(result["claim_scope"], gate.CLAIM_SCOPE)

    def test_existing_complete_verifier_fixture_has_real_six_field_rows(self) -> None:
        fixture_script = Path(__file__).with_name("test_archive_supply_chain_evidence_verifier.py")
        spec = importlib.util.spec_from_file_location("archive_verifier_complete_fixture", fixture_script)
        fixture_module = importlib.util.module_from_spec(spec)
        assert spec and spec.loader
        sys.modules[spec.name] = fixture_module
        spec.loader.exec_module(fixture_module)
        with tempfile.TemporaryDirectory(prefix="archive-real-index-") as temporary:
            fixture = fixture_module.CompleteEvidenceFixture(Path(temporary).resolve())
            verified = fixture_module.verifier.verify_profile(fixture.args())
            self.assertTrue(verified["valid"])
            self.assertEqual(verified["archive_count"], 7)
            self.assertEqual(verified["spdx_document_count"], 8)
            index_bytes, rows = gate.load_index_rows(fixture.bundle, fixture.commit)
            self.assertEqual(index_bytes, fixture.index_bytes)
            self.assertEqual(len(rows), len(fixture.rows))
            self.assertTrue(all(set(row) == {"ecosystem", "kind", "path", "bytes", "sha256", "builder"}
                                for row in rows))
            self.assertTrue(all(row["builder"] == fixture.rows[position]["builder"]
                                for position, row in enumerate(rows)))

    def test_index_builder_metadata_must_match_build_receipt(self) -> None:
        modified = json.loads(self.index_bytes)
        modified["artifacts"][0]["builder"]["platform"] = "forged"
        (self.bundle / "ARCHIVE-INDEX.json").write_text(json.dumps(modified))
        with self.assertRaisesRegex(gate.GateError, "archive_index_builder_mismatch"):
            gate.load_index_rows(self.bundle, self.source)

    def test_index_missing_builder_field_is_rejected(self) -> None:
        modified = json.loads(self.index_bytes)
        del modified["artifacts"][0]["builder"]
        (self.bundle / "ARCHIVE-INDEX.json").write_text(json.dumps(modified))
        with self.assertRaisesRegex(gate.GateError, "archive_index_row_shape"):
            gate.load_index_rows(self.bundle, self.source)

    def test_verifier_failure_prevents_builder_and_output(self) -> None:
        calls, runner = self._runner(verifier_code=1)
        with mock.patch.object(gate, "run_bounded", side_effect=runner):
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        self.assertEqual(len(calls), 1)
        self.assertFalse(self.output.exists())

    def test_unexpected_verifier_success_shape_prevents_builder(self) -> None:
        calls, runner = self._runner()

        def malformed(argv: list[str], process_runner=None):
            code, out, err = runner(argv, process_runner)
            if argv[1].endswith(gate.VERIFIER):
                value = json.loads(out)
                value["unexpected"] = True
                return 0, json.dumps(value).encode(), err
            return code, out, err

        with mock.patch.object(gate, "run_bounded", side_effect=malformed):
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        self.assertEqual(len(calls), 1)
        self.assertFalse(self.output.exists())

    def test_verifier_pin_mismatch_fails_before_subprocess(self) -> None:
        self.args.expected_verifier_sha256 = "0" * 64
        with mock.patch.object(gate, "run_bounded") as run:
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        run.assert_not_called()

    def test_helper_pin_mismatch_fails_before_subprocess(self) -> None:
        inputs = json.loads(self.inputs.read_text())
        target = "packaging/scripts/" + gate.HELPERS[0]
        next(row for row in inputs["dependencies"] if row["id"] == target)["sha256"] = "0" * 64
        self.inputs.write_text(json.dumps(inputs))
        with mock.patch.object(gate, "run_bounded") as run:
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        run.assert_not_called()

    def test_external_expectation_inside_evidence_is_rejected(self) -> None:
        self.args.expected_inputs = self.evidence / "expected-inputs.json"
        self.args.expected_inputs.write_text(self.inputs.read_text())
        with mock.patch.object(gate, "run_bounded") as run:
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        run.assert_not_called()

    def test_source_binding_mismatch_fails_before_subprocess(self) -> None:
        self.args.release_source_commit = "2" * 40
        with mock.patch.object(gate, "run_bounded") as run:
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        run.assert_not_called()

    def test_archive_zip_outside_acquisition_root_is_rejected(self) -> None:
        external_zip = self.external / "artifact.zip"
        external_zip.write_bytes(self.archive_zip.read_bytes())
        self.args.archive_zip = external_zip
        with mock.patch.object(gate, "run_bounded") as run:
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        run.assert_not_called()

    def test_extra_generated_file_fails_and_new_output_is_removed(self) -> None:
        calls, runner = self._runner(extra=True)
        with mock.patch.object(gate, "run_bounded", side_effect=runner):
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        self.assertEqual(len(calls), 2)
        self.assertFalse(self.output.exists())

    def test_tampered_copied_archive_fails_and_new_output_is_removed(self) -> None:
        calls, runner = self._runner()

        def tampered(argv: list[str], process_runner=None):
            code, out, err = runner(argv, process_runner)
            if argv[1].endswith("build_archive_release_manifest.py"):
                staged_output = Path(argv[argv.index("--output") + 1])
                archive = staged_output / "archives" / self.row["path"]
                archive.write_bytes(b"changed after copy")
            return code, out, err

        with mock.patch.object(gate, "run_bounded", side_effect=tampered):
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        self.assertEqual(len(calls), 2)
        self.assertFalse(self.output.exists())

    def test_rehashed_manifest_subject_omission_fails(self) -> None:
        calls, runner = self._runner()

        def missing_subject(argv: list[str], process_runner=None):
            code, out, err = runner(argv, process_runner)
            if argv[1].endswith("build_archive_release_manifest.py"):
                staged = Path(argv[argv.index("--output") + 1])
                manifest_path = staged / "release-artifact-manifest.json"
                manifest = json.loads(manifest_path.read_text())
                manifest["artifacts"].pop()
                manifest_path.write_text(json.dumps(manifest))
                sums = "".join(f"{row['sha256']}  {row['path']}\n" for row in manifest["artifacts"])
                (staged / "SHA256SUMS").write_text(sums)
            return code, out, err

        with mock.patch.object(gate, "run_bounded", side_effect=missing_subject):
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        self.assertEqual(len(calls), 2)
        self.assertFalse(self.output.exists())

    def test_rehashed_false_manifest_digest_fails(self) -> None:
        calls, runner = self._runner()

        def false_digest(argv: list[str], process_runner=None):
            code, out, err = runner(argv, process_runner)
            if argv[1].endswith("build_archive_release_manifest.py"):
                staged = Path(argv[argv.index("--output") + 1])
                manifest_path = staged / "release-artifact-manifest.json"
                manifest = json.loads(manifest_path.read_text())
                manifest["artifacts"][0]["sha256"] = "f" * 64
                manifest_path.write_text(json.dumps(manifest))
                sums = "".join(f"{row['sha256']}  {row['path']}\n" for row in manifest["artifacts"])
                (staged / "SHA256SUMS").write_text(sums)
            return code, out, err

        with mock.patch.object(gate, "run_bounded", side_effect=false_digest):
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        self.assertEqual(len(calls), 2)
        self.assertFalse(self.output.exists())

    def test_bundle_source_mismatch_prevents_verifier(self) -> None:
        index = {"schema_version": 1, "source_commit": "2" * 40,
                 "created_at_utc": "2026-10-06T00:00:00Z", "artifacts": self.rows}
        self.index_bytes = json.dumps(index, separators=(",", ":")).encode()
        (self.bundle / "ARCHIVE-INDEX.json").write_bytes(self.index_bytes)
        with mock.patch.object(gate, "run_bounded") as run:
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        run.assert_not_called()

    def test_existing_output_is_never_overwritten(self) -> None:
        self.output.mkdir()
        marker = self.output / "preserve.txt"
        marker.write_text("preserve")
        with mock.patch.object(gate, "run_bounded") as run:
            with self.assertRaises(gate.GateError):
                gate.prepare(self.args)
        run.assert_not_called()
        self.assertEqual(marker.read_text(), "preserve")

    def test_bounded_child_timeout_and_output_cap_and_held_stdout(self) -> None:
        python = sys.executable
        process_runner = gate.load_acquisition_runner(
            (gate.SCRIPT_DIR / "acquire_package_archive_bundle.py").read_bytes())
        with mock.patch.object(gate, "MAX_SUBPROCESS_SECONDS", 0.05):
            with self.assertRaisesRegex(gate.GateError, "subprocess_timeout"):
                gate.run_bounded([python, "-c", "import time; time.sleep(2)"], process_runner)
            with self.assertRaisesRegex(gate.GateError, "subprocess_timeout"):
                gate.run_bounded([python, "-c", "import subprocess,sys; subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(2)'])"], process_runner)
        with mock.patch.object(gate, "MAX_SUBPROCESS_OUTPUT", 32):
            with self.assertRaisesRegex(gate.GateError, "subprocess_output_limit"):
                gate.run_bounded([python, "-c", "print('x' * 1000)"], process_runner)

    def test_live_process_group_permission_error_is_not_ignored(self) -> None:
        process_runner = gate.load_acquisition_runner(
            (gate.SCRIPT_DIR / "acquire_package_archive_bundle.py").read_bytes())

        class LiveProcess:
            pid = 12345

            @staticmethod
            def poll():
                return None

        with mock.patch.object(process_runner.__globals__["os"], "killpg", side_effect=PermissionError):
            with self.assertRaises(PermissionError):
                process_runner.__globals__["_signal_process_group"](LiveProcess(), process_runner.__globals__["signal"].SIGTERM)

    def test_competing_empty_output_created_during_install_is_preserved(self) -> None:
        calls, runner = self._runner()

        def competitor(argv: list[str], process_runner=None):
            result = runner(argv, process_runner)
            if argv[1].endswith("build_archive_release_manifest.py"):
                self.output.mkdir()
                (self.output / "competitor.marker").write_text("keep")
            return result

        with mock.patch.object(gate, "run_bounded", side_effect=competitor):
            with self.assertRaisesRegex(gate.GateError, "output_appeared_during_gate"):
                gate.prepare(self.args)
        self.assertEqual(len(calls), 2)
        self.assertEqual((self.output / "competitor.marker").read_text(), "keep")


if __name__ == "__main__":
    unittest.main()

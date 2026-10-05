from __future__ import annotations

import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import struct
import tempfile
import unittest
from unittest import mock
import warnings
import zipfile


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "packaging/scripts/acquire_archive_consumer_evidence.py"
SPEC = importlib.util.spec_from_file_location("consumer_acquisition", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
acq = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(acq)


SOURCE = "30d3c38e9740e316c1cf6046cbce0ee8aae9d796"
PRODUCER_SOURCE = "5bdc1b42d2e3ad4722f8317e19be92aef268f815"
RUN_ID = 37318162611
ATTEMPT = 2
ARTIFACT_ID = 11349051447
REPOSITORY_ID = 123456


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True) + "\n").encode()


def consumer_layout() -> dict[str, bytes]:
    evidence_root, acquisition_root, syft_root = acq._consumer_prefixes(RUN_ID, ATTEMPT)
    prefix = evidence_root + "/"
    rows: dict[str, bytes] = {
        prefix + "preparation-receipt.json": json_bytes({
            "kind": "archive-evidence-expectations-preparation",
            "trusted_consumer_sha": SOURCE,
            "producer_pins": {"repository": acq.REPOSITORY, "source_commit": PRODUCER_SOURCE,
                              "run_id": 37318162610, "artifact_id": 11349051446},
            "prepared_files": {"acquisition.json": "a" * 64, "expected-inputs.json": "b" * 64,
                                "outer-binding.json": "c" * 64},
        }),
        prefix + "expectations/acquisition.json": json_bytes({"kind": "derived", "source_commit": PRODUCER_SOURCE}),
        prefix + "expectations/expected-inputs.json": json_bytes({
            "source_commit": PRODUCER_SOURCE, "original_run_id": 37318162610,
            "acquisition_artifact_id": 11318162610, "archive_index_sha256": "d" * 64,
            "dependencies": [],
        }),
        prefix + "expectations/outer-binding.json": json_bytes({"schema_version": 1, "source_commit": SOURCE}),
        prefix + "validation-report.json": json_bytes({
            "valid": True, "profile": acq.PROFILE, "archive_count": 8, "ecosystem_count": 7,
            "spdx_document_count": 9, "evidence_file_count": 44,
            "archive_index_sha256": "d" * 64, "statement_sha256": "e" * 64,
            "claim_scope": acq.CLAIM_SCOPE,
        }),
    }
    for name in acq.EVIDENCE_ROOT_FILES:
        rows[prefix + "evidence/" + name] = b"fixture\n"
    for name in acq.BUILD_INPUT_FILES:
        rows[prefix + "evidence/build-inputs/" + name] = b"{}\n"
    archives = (
        ("rust", "a.crate"), ("python", "a.whl"), ("python", "a.tar.gz"), ("r", "a.tar.gz"),
        ("julia", "a.tar.gz"), ("typescript", "a.tgz"), ("nuget", "a.nupkg"), ("go", "a.tar.gz"),
    )
    for ecosystem, name in archives:
        rows[prefix + f"evidence/archives/{ecosystem}/{name}"] = b"archive bytes\n"
    for index in range(8):
        digest = f"{index:064x}"
        for suffix in ("spdx.json", "stdout", "stderr"):
            rows[prefix + f"evidence/component-sboms/{digest}.{suffix}"] = b"component\n"

    acquisition_prefix = acquisition_root + "/"
    for name in ("receipt.json", "artifact-metadata.json", "run-metadata.json", "source-commit-readback.json"):
        rows[acquisition_prefix + name] = json_bytes({"fixture": name})
    syft_prefix = syft_root + "/"
    syft_receipt = json_bytes({"schema": "kairos.verified-syft-installer.v1", "result": "pass",
                               "target": "linux-amd64", "platform": "linux/amd64", "version": "1.54.0"})
    rows[syft_prefix + "evidence/receipt.json"] = syft_receipt
    rows[syft_prefix + "evidence/validation-report.json"] = json_bytes(
        {"schema": "kairos.syft-installation-validation.v1", "result": "pass",
         "target": "linux-amd64", "platform": "linux/amd64",
         "receipt_sha256": hashlib.sha256(syft_receipt).hexdigest(), "binary_sha256": "3" * 64})
    rows[syft_prefix + "logs/install.log"] = b"verified installer fixture\n"
    rows[prefix + "expectations/expected-inputs.json"] = json_bytes({
        "archive_index_sha256": "d" * 64,
        "source_commit": PRODUCER_SOURCE,
        "original_run_id": 37318162610,
        "acquisition_artifact_id": 11318162610,
        "dependencies": [{"id": f"dep:{index}", "sha256": f"{index:064x}"} for index in range(12)],
    })
    outer = {
        "acquisition_artifact_id": 11318162610, "archive_zip_bytes": 57381,
        "archive_zip_sha256": "f" * 64, "original_run_id": 37318162610,
        "producer_pr_head": PRODUCER_SOURCE, "producer_tree": "1" * 40,
        "repository": acq.REPOSITORY, "schema_version": 1,
        "source_commit": PRODUCER_SOURCE, "spdx_schema_sha256": "2" * 64,
    }
    rows[prefix + "expectations/outer-binding.json"] = json_bytes(outer)
    derived = {
        "archive_count": 8, "archive_index_sha256": "d" * 64,
        "artifact_digest": "sha256:" + "f" * 64, "artifact_id": 11318162610,
        "derivation": {"inputs": {}, "status": "derived local adapter receipt; not original acquisition history"},
        "ecosystems": ["go", "julia", "nuget", "python", "r", "rust", "typescript"],
        "repository": acq.REPOSITORY, "run_id": 37318162610, "source_commit": PRODUCER_SOURCE,
    }
    rows[prefix + "expectations/acquisition.json"] = json_bytes(derived)
    prep = {
        "kind": "archive-evidence-expectations-preparation", "schema_version": 1,
        "trusted_consumer_sha": SOURCE, "producer_pins": {
            "repository": acq.REPOSITORY, "source_commit": PRODUCER_SOURCE, "producer_tree": "1" * 40,
            "run_id": 37318162610, "artifact_id": 11318162610,
            "archive_zip_sha256": "f" * 64, "archive_zip_bytes": 57381,
        },
        "spdx_schema_sha256": "2" * 64,
        "syft_qualification": {"receipt_sha256": hashlib.sha256(syft_receipt).hexdigest(),
                               "target": "linux-amd64",
                               "binary_sha256": "3" * 64},
        "prepared_files": {
            "acquisition.json": hashlib.sha256(rows[prefix + "expectations/acquisition.json"]).hexdigest(),
            "expected-inputs.json": hashlib.sha256(rows[prefix + "expectations/expected-inputs.json"]).hexdigest(),
            "outer-binding.json": hashlib.sha256(rows[prefix + "expectations/outer-binding.json"]).hexdigest(),
        },
    }
    expected_inputs = json.loads(rows[prefix + "expectations/expected-inputs.json"])
    expected_inputs["dependencies"][-1] = {"id": "tool:syft", "sha256": "3" * 64}
    rows[prefix + "expectations/expected-inputs.json"] = json_bytes(expected_inputs)
    prep["prepared_files"]["expected-inputs.json"] = hashlib.sha256(rows[prefix + "expectations/expected-inputs.json"]).hexdigest()
    rows[prefix + "preparation-receipt.json"] = json_bytes(prep)
    return rows


def zip_bytes(files: dict[str, bytes], extra: tuple[str, bytes] | None = None,
              symlink: str | None = None) -> bytes:
    stream = io.BytesIO()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", UserWarning)
        with zipfile.ZipFile(stream, "w", compression=zipfile.ZIP_DEFLATED) as zipped:
            for path, data in files.items():
                zipped.writestr(path, data)
            if extra is not None:
                zipped.writestr(extra[0], extra[1])
            if symlink is not None:
                info = zipfile.ZipInfo(symlink)
                info.create_system = 3
                info.external_attr = (stat.S_IFLNK | 0o777) << 16
                zipped.writestr(info, b"target")
    return stream.getvalue()


class ConsumerEvidenceAcquisitionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.payload = consumer_layout()
        self.archive = zip_bytes(self.payload)
        self.sha = hashlib.sha256(self.archive).hexdigest()
        self.pins = {
            "run_id": RUN_ID, "run_attempt": ATTEMPT, "artifact_id": ARTIFACT_ID,
            "source_commit": SOURCE, "archive_zip_sha256": self.sha,
            "archive_zip_bytes": len(self.archive),
            "expected_helper_sha256": hashlib.sha256(
                (ROOT / "packaging/scripts/acquire_package_archive_bundle.py").read_bytes()
            ).hexdigest(),
        }
        self.run = {
            "id": RUN_ID, "run_attempt": ATTEMPT, "path": acq.WORKFLOW,
            "status": "completed", "conclusion": "success", "event": "workflow_dispatch",
            "head_branch": "main", "head_sha": SOURCE, "pull_requests": [],
            "repository": {"id": REPOSITORY_ID, "full_name": acq.REPOSITORY},
            "head_repository": {"id": REPOSITORY_ID, "full_name": acq.REPOSITORY},
        }
        self.artifact = {
            "id": ARTIFACT_ID, "name": f"archive-main-evidence-{RUN_ID}-{ATTEMPT}",
            "expired": False, "digest": "sha256:" + self.sha, "size_in_bytes": len(self.archive),
            "workflow_run": {"id": RUN_ID, "repository_id": REPOSITORY_ID,
                             "head_repository_id": REPOSITORY_ID, "head_sha": SOURCE,
                             "head_branch": "main"},
        }
        self.api_rows = {
            f"repos/{acq.REPOSITORY}/actions/runs/{RUN_ID}/attempts/{ATTEMPT}": self.run,
            f"repos/{acq.REPOSITORY}/actions/artifacts/{ARTIFACT_ID}": self.artifact,
        }

    def tearDown(self) -> None:
        self.temp.cleanup()

    def fetch(self, _helper: object, endpoint: str) -> tuple[bytes, dict]:
        return json_bytes(self.api_rows[endpoint]), self.api_rows[endpoint]

    def download(self, _helper: object, endpoint: str, destination: Path) -> str:
        self.assertEqual(endpoint, f"repos/{acq.REPOSITORY}/actions/artifacts/{ARTIFACT_ID}/zip")
        destination.write_bytes(self.archive)
        return "sha256:" + self.sha

    def run_acquisition(self, *, output: Path | None = None) -> dict:
        return acq.acquire(**self.pins, output=output or self.root / "retained",
                           api_fetch=self.fetch, download=self.download)

    def test_complete_main_consumer_artifact_is_retained_with_native_records_and_receipt(self) -> None:
        output = self.root / "retained"
        receipt = self.run_acquisition(output=output)
        self.assertEqual(receipt["schema"], "kairos.archive-consumer-evidence-acquisition.v1")
        self.assertEqual(receipt["run_attempt"], ATTEMPT)
        self.assertEqual(receipt["consumer_validation"]["valid"], True)
        self.assertEqual((output / f"{ARTIFACT_ID}.zip").read_bytes(), self.archive)
        self.assertEqual(json.loads((output / "run-metadata.json").read_text()), self.run)
        self.assertEqual(json.loads((output / "artifact-metadata.json").read_text()), self.artifact)
        stored = json.loads((output / "receipt.json").read_text())
        self.assertEqual(stored, receipt)
        self.assertTrue((output / "evidence" / f"archive-evidence-{RUN_ID}-{ATTEMPT}" / "evidence" / "validation-result.json").is_file())
        self.assertEqual(len(receipt["extracted_files"]), len(self.payload))

    def test_distinct_consumer_and_producer_sources_are_bound_across_all_maps(self) -> None:
        evidence_root, _, _ = acq._consumer_prefixes(RUN_ID, ATTEMPT)
        for rel, key in (("expectations/outer-binding.json", "source_commit"),
                         ("expectations/expected-inputs.json", "source_commit"),
                         ("expectations/acquisition.json", "source_commit")):
            rows = dict(self.payload)
            path = f"{evidence_root}/{rel}"
            document = json.loads(rows[path])
            document[key] = SOURCE
            rows[path] = json_bytes(document)
            prep_path = f"{evidence_root}/preparation-receipt.json"
            prep = json.loads(rows[prep_path])
            prepared_name = rel.rsplit("/", 1)[-1]
            prep["prepared_files"][prepared_name] = hashlib.sha256(rows[path]).hexdigest()
            rows[prep_path] = json_bytes(prep)
            with tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                for name, data in rows.items():
                    target = root / name
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(data)
                with self.subTest(path=rel), self.assertRaises(acq.AcquisitionError):
                    acq.validate_consumer_layout(root, RUN_ID, ATTEMPT, SOURCE)

    def test_syft_target_metadata_must_match_linux_amd64_namespace(self) -> None:
        evidence_root, _, syft_root = acq._consumer_prefixes(RUN_ID, ATTEMPT)
        cases = (
            (f"{evidence_root}/preparation-receipt.json", "syft_qualification", "target"),
            (f"{syft_root}/evidence/receipt.json", "target", None),
            (f"{syft_root}/evidence/validation-report.json", "target", None),
        )
        for path, key, nested in cases:
            rows = dict(self.payload)
            value = json.loads(rows[path])
            if nested is None:
                value[key] = "darwin-arm64"
            else:
                value[key][nested] = "darwin-arm64"
            rows[path] = json_bytes(value)
            prep_path = f"{evidence_root}/preparation-receipt.json"
            prep = json.loads(rows[prep_path])
            if path.endswith("receipt.json") and path.startswith(syft_root):
                prep["syft_qualification"]["receipt_sha256"] = hashlib.sha256(rows[path]).hexdigest()
            rows[prep_path] = json_bytes(prep)
            with tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                for name, data in rows.items():
                    target = root / name
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(data)
                with self.subTest(path=path), self.assertRaises(acq.AcquisitionError):
                    acq.validate_consumer_layout(root, RUN_ID, ATTEMPT, SOURCE)

    def test_syft_validation_report_binds_receipt_and_binary(self) -> None:
        _, _, syft_root = acq._consumer_prefixes(RUN_ID, ATTEMPT)
        path = f"{syft_root}/evidence/validation-report.json"
        for key, bad in (("schema", "other.schema"), ("receipt_sha256", "0" * 64),
                         ("binary_sha256", "0" * 64)):
            rows = dict(self.payload)
            value = json.loads(rows[path])
            value[key] = bad
            rows[path] = json_bytes(value)
            with tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                for name, data in rows.items():
                    target = root / name
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(data)
                with self.subTest(key=key), self.assertRaises(acq.AcquisitionError):
                    acq.validate_consumer_layout(root, RUN_ID, ATTEMPT, SOURCE)

    def test_helper_hash_is_checked_before_any_api_or_download(self) -> None:
        self.pins["expected_helper_sha256"] = "0" * 64
        with self.assertRaisesRegex(acq.AcquisitionError, "acquisition_helper_pin_mismatch"):
            self.run_acquisition()

    def test_native_api_json_rejects_duplicates_nonfinite_and_oversize(self) -> None:
        helper = type("Helper", (), {
            "_run_bounded_process": staticmethod(lambda *_: ("sha256:" + "0" * 64, b'{"id":1,"id":2}')),
            "API_COMMAND_PREFIX": ["gh", "api"], "API_OUTPUT_LIMIT": 16 * 1024 * 1024,
            "API_TIMEOUT_SECONDS": 60,
        })
        with self.assertRaisesRegex(acq.AcquisitionError, "api_readback_duplicate_key"):
            acq.api_readback(helper, f"repos/{acq.REPOSITORY}/actions/test")
        with self.assertRaisesRegex(acq.AcquisitionError, "api_readback_non_finite"):
            acq.strict_json(b'{"value":1e999}', "api_readback")
        with self.assertRaisesRegex(acq.AcquisitionError, "api_readback_too_large"):
            acq.strict_json(b" " * (acq.MAX_JSON_BYTES + 1), "api_readback")

    def test_default_path_uses_the_pinned_bounded_argv_process_runner(self) -> None:
        helper = acq.load_captured_helper(self.pins["expected_helper_sha256"])
        observed = []

        def bounded(argv, limit, timeout, output_path=None):
            observed.append((argv, limit, timeout, output_path))
            endpoint = argv[-1]
            if output_path is not None:
                output_path.write_bytes(self.archive)
                return "sha256:" + self.sha, None
            return "sha256:" + "0" * 64, json_bytes(self.api_rows[endpoint])

        helper._run_bounded_process = bounded
        output = self.root / "retained-bounded"
        with mock.patch.object(acq, "load_captured_helper", return_value=helper):
            result = acq.acquire(**self.pins, output=output)
        self.assertEqual(result["artifact_id"], ARTIFACT_ID)
        self.assertEqual(len(observed), 3)
        self.assertEqual(observed[0][0], ["gh", "api", f"repos/{acq.REPOSITORY}/actions/runs/{RUN_ID}/attempts/{ATTEMPT}"])
        self.assertEqual(observed[1][0], ["gh", "api", f"repos/{acq.REPOSITORY}/actions/artifacts/{ARTIFACT_ID}"])
        self.assertEqual(observed[2][0], ["gh", "api", f"repos/{acq.REPOSITORY}/actions/artifacts/{ARTIFACT_ID}/zip"])
        self.assertTrue(all(0 < row[1] <= 128 * 1024 * 1024 for row in observed))
        self.assertTrue(all(0 < row[2] <= 300 for row in observed))

    def test_run_status_event_path_attempt_source_and_repository_fail_closed(self) -> None:
        cases = (
            ("status", "in_progress"), ("conclusion", "failure"), ("event", "push"),
            ("head_branch", "feature"), ("path", ".github/workflows/other.yml"),
            ("run_attempt", ATTEMPT + 1), ("head_sha", "a" * 40),
        )
        for field, value in cases:
            with self.subTest(field=field):
                self.api_rows[next(iter(self.api_rows))] = {**self.run, field: value}
                with self.assertRaises(acq.AcquisitionError):
                    self.run_acquisition()
        self.api_rows[next(iter(self.api_rows))] = {
            **self.run, "repository": {"id": REPOSITORY_ID, "full_name": "attacker/repo"}}
        with self.assertRaisesRegex(acq.AcquisitionError, "consumer_run_repository_mismatch"):
            self.run_acquisition()

    def test_artifact_identity_name_digest_size_expiry_and_origin_are_pinned(self) -> None:
        original = self.artifact
        bad_rows = (
            {**original, "id": ARTIFACT_ID + 1},
            {**original, "name": "archive-main-evidence-latest"},
            {**original, "expired": True},
            {**original, "digest": "sha256:" + "0" * 64},
            {**original, "size_in_bytes": len(self.archive) + 1},
            {**original, "workflow_run": {**original["workflow_run"], "id": RUN_ID + 1}},
            {**original, "workflow_run": {**original["workflow_run"], "head_repository_id": REPOSITORY_ID + 1}},
            {**original, "workflow_run": {**original["workflow_run"], "head_sha": "a" * 40}},
            {**original, "workflow_run": {**original["workflow_run"], "head_branch": "release"}},
        )
        key = list(self.api_rows)[1]
        for bad in bad_rows:
            with self.subTest(artifact=bad):
                self.api_rows[key] = bad
                with self.assertRaises(acq.AcquisitionError):
                    self.run_acquisition()
        self.api_rows[key] = original

    def test_download_zip_digest_and_byte_length_are_rechecked(self) -> None:
        with mock.patch.object(self, "download", side_effect=lambda _h, _e, path: (path.write_bytes(self.archive), "sha256:" + "0" * 64)[1]):
            with self.assertRaisesRegex(acq.AcquisitionError, "consumer_zip_download_digest_mismatch"):
                self.run_acquisition()
        with mock.patch.object(self, "download", side_effect=lambda _h, _e, path: (path.write_bytes(self.archive + b"x"), "sha256:" + self.sha)[1]):
            with self.assertRaisesRegex(acq.AcquisitionError, "consumer_zip_download_size_mismatch"):
                self.run_acquisition()
        self.pins["archive_zip_sha256"] = "0" * 64
        with self.assertRaises(acq.AcquisitionError):
            self.run_acquisition()
        self.pins["archive_zip_sha256"] = self.sha
        self.pins["archive_zip_bytes"] += 1
        with self.assertRaises(acq.AcquisitionError):
            self.run_acquisition()

    def test_traversal_case_alias_duplicate_symlink_and_unrecognized_extra_reject(self) -> None:
        cases = (
            zip_bytes(self.payload, extra=("../outside.txt", b"no")),
            zip_bytes(self.payload, extra=("EXTRA.txt", b"no")),
            zip_bytes(self.payload, extra=("Archive-Evidence-37318162611-2/x", b"case alias")),
            zip_bytes(self.payload, extra=("archive-evidence-37318162611-2/evidence/release.txt", b"case alias")),
            zip_bytes(self.payload, extra=(next(iter(self.payload)), b"duplicate")),
            zip_bytes(self.payload, extra=("archive-evidence-37318162611-2/evidence/component-sboms/extra.pdf", b"extra")),
            zip_bytes(self.payload, extra=(f"syft-linux-amd64-{RUN_ID}-{ATTEMPT}/logs/nested/extra.log", b"nested log")),
            zip_bytes(self.payload, symlink="archive-evidence-37318162611-2/evidence/link"),
        )
        for archive in cases:
            with self.subTest(sha=hashlib.sha256(archive).hexdigest()):
                self.archive = archive
                self.sha = hashlib.sha256(archive).hexdigest()
                self.pins["archive_zip_sha256"] = self.sha
                self.pins["archive_zip_bytes"] = len(archive)
                self.artifact = {**self.artifact, "digest": "sha256:" + self.sha,
                                 "size_in_bytes": len(archive)}
                self.api_rows[list(self.api_rows)[1]] = self.artifact
                with self.assertRaises(acq.AcquisitionError):
                    self.run_acquisition()
                self.assertFalse((self.root / "retained").exists())

    def test_zip64_and_excessive_central_directory_count_reject_before_zipfile(self) -> None:
        signature = b"PK\x05\x06"
        position = self.archive.rfind(signature)
        self.assertGreaterEqual(position, 0)
        for disk_entries, total_entries in ((acq.MAX_MEMBERS + 1, acq.MAX_MEMBERS + 1), (0xFFFF, 0xFFFF)):
            damaged = bytearray(self.archive)
            struct.pack_into("<H", damaged, position + 8, disk_entries)
            struct.pack_into("<H", damaged, position + 10, total_entries)
            path = self.root / "bad.zip"
            path.write_bytes(damaged)
            with self.subTest(entries=total_entries), mock.patch.object(acq.zipfile, "ZipFile") as zipfile_open:
                with self.assertRaises(acq.AcquisitionError):
                    acq.preflight_zip(path, hashlib.sha256(damaged).hexdigest(), len(damaged))
                zipfile_open.assert_not_called()

    def test_zip_file_cannot_be_ancestor_of_explicit_directory(self) -> None:
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, "w") as zipped:
            zipped.writestr("foo", b"file")
            zipped.writestr("foo/bar/", b"")
        damaged = stream.getvalue()
        path = self.root / "collision.zip"
        path.write_bytes(damaged)
        with self.assertRaisesRegex(acq.AcquisitionError, "consumer_zip_file_directory_conflict"):
            acq.preflight_zip(path, hashlib.sha256(damaged).hexdigest(), len(damaged))

    def test_invalid_consumer_profile_and_missing_layout_file_reject(self) -> None:
        report_path = f"archive-evidence-{RUN_ID}-{ATTEMPT}/validation-report.json"
        baseline = self.payload
        invalid_report = {**baseline, report_path: json_bytes({"valid": False})}
        missing = dict(baseline)
        missing.pop(f"archive-evidence-{RUN_ID}-{ATTEMPT}/evidence/validation-result.json")
        extra = {**baseline, f"archive-evidence-{RUN_ID}-{ATTEMPT}/unexpected.txt": b"extra"}
        for files in (invalid_report, missing, extra):
            with self.subTest(file_count=len(files)):
                self.archive = zip_bytes(files)
                self.sha = hashlib.sha256(self.archive).hexdigest()
                self.pins.update(archive_zip_sha256=self.sha, archive_zip_bytes=len(self.archive))
                self.artifact = {**self.artifact, "digest": "sha256:" + self.sha, "size_in_bytes": len(self.archive)}
                self.api_rows[list(self.api_rows)[1]] = self.artifact
                with self.assertRaises(acq.AcquisitionError):
                    self.run_acquisition()
                self.assertFalse((self.root / "retained").exists())

    def test_existing_output_and_raced_empty_destination_are_never_replaced(self) -> None:
        existing = self.root / "retained"
        existing.mkdir()
        marker = existing / "marker"
        marker.write_text("competitor")
        with self.assertRaisesRegex(acq.AcquisitionError, "output_already_exists"):
            self.run_acquisition(output=existing)
        self.assertEqual(marker.read_text(), "competitor")

        stage = self.root / "stage"
        stage.mkdir()
        (stage / "receipt.json").write_text("ours")
        race = self.root / "race"
        race.mkdir()
        (race / "marker").write_text("competitor")
        parent_fd = os.open(self.root, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        stage_fd = os.open(stage, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        try:
            with self.assertRaises(acq.AcquisitionError):
                acq._move_into_exclusive_output(stage_fd, parent_fd, race.name)
        finally:
            os.close(stage_fd)
            os.close(parent_fd)
        self.assertEqual((race / "marker").read_text(), "competitor")
        self.assertEqual((stage / "receipt.json").read_text(), "ours")

    def test_output_parent_symlink_is_rejected(self) -> None:
        real = self.root / "real"
        real.mkdir()
        link = self.root / "alias"
        link.symlink_to(real, target_is_directory=True)
        with self.assertRaisesRegex(acq.AcquisitionError, "output_parent_invalid"):
            self.run_acquisition(output=link / "out")

    def test_renamed_output_parent_is_anchored_and_competitor_preserved(self) -> None:
        parent = self.root / "owned-parent"
        parent.mkdir()
        moved = self.root / "renamed-parent"
        replacement = parent / "attacker-marker"

        def swap_parent(_helper: object, _endpoint: str, destination: Path) -> str:
            destination.write_bytes(self.archive)
            parent.rename(moved)
            parent.mkdir()
            replacement.write_text("competitor")
            return "sha256:" + self.sha

        with mock.patch.object(self, "download", side_effect=swap_parent):
            with self.assertRaisesRegex(acq.AcquisitionError, "output_parent_changed_during_install"):
                self.run_acquisition(output=parent / "retained")
        self.assertEqual(replacement.read_text(), "competitor")
        self.assertFalse((moved / "retained").exists())
        self.assertEqual(list(moved.iterdir()), [])

    def test_replaced_output_entry_survives_success_and_error_cleanup(self) -> None:
        original_move = acq._move_into_exclusive_output
        output = self.root / "retained"
        backup = self.root / "moved-owned-output"

        def replace_after_install(stage_fd: int, parent_fd: int, name: str):
            result = original_move(stage_fd, parent_fd, name)
            output.rename(backup)
            output.mkdir()
            (output / "competitor").write_text("keep")
            return result

        with mock.patch.object(acq, "_move_into_exclusive_output", side_effect=replace_after_install):
            with self.assertRaisesRegex(acq.AcquisitionError, "output_reservation_replaced"):
                self.run_acquisition(output=output)
        self.assertEqual((output / "competitor").read_text(), "keep")
        self.assertTrue((backup / "receipt.json").is_file())

        stage = self.root / "stage-error"
        stage.mkdir()
        (stage / "our-file").write_text("owned")
        parent_fd = os.open(self.root, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        stage_fd = os.open(stage, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        real_rename = os.rename

        def move_then_replace(src, dst, *, src_dir_fd=None, dst_dir_fd=None):
            result = real_rename(src, dst, src_dir_fd=src_dir_fd, dst_dir_fd=dst_dir_fd)
            if src == "our-file":
                real_rename("error-output", "moved-error-output", src_dir_fd=parent_fd, dst_dir_fd=parent_fd)
                os.mkdir("error-output", dir_fd=parent_fd)
                (self.root / "error-output" / "competitor").write_text("keep")
                raise OSError("injected post-install failure")
            return result

        try:
            with mock.patch.object(acq.os, "rename", side_effect=move_then_replace):
                with self.assertRaisesRegex(OSError, "injected post-install"):
                    acq._move_into_exclusive_output(stage_fd, parent_fd, "error-output")
        finally:
            os.close(stage_fd)
            os.close(parent_fd)
        self.assertEqual((self.root / "error-output" / "competitor").read_text(), "keep")


if __name__ == "__main__":
    unittest.main()

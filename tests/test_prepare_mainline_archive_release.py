from __future__ import annotations

import argparse
import base64
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock
import zlib

SCRIPT = Path(__file__).resolve().parents[1] / "packaging/scripts/prepare_mainline_archive_release.py"
SPEC = importlib.util.spec_from_file_location("prepare_mainline_archive_release", SCRIPT)
gate = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(gate)


SHA = "a" * 64
COMMIT = "b" * 40

NATIVE_SYFT_FIXTURE_SHA256 = "0a9b35fca967b6ecf130670c47b4e84331bbed1d7d7b76b81033561453752bc9"
NATIVE_SYFT_FIXTURE_ZLIB_B64 = (
    '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'
    'HprMnINw9jrqs6Hyh4f/AvyTwMk='
)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, indent=2) + "\n").encode()


class MainlineArchivePreparationTests(unittest.TestCase):
    def test_command_credentials_are_limited_to_acquisition_children_and_restored(self) -> None:
        with tempfile.TemporaryDirectory() as temp, mock.patch.dict(os.environ, {"GH_TOKEN": "secret", "ACTIONS_RUNTIME_TOKEN": "runtime"}):
            work = Path(temp).resolve()
            (work / "logs").mkdir()
            seen = []

            def runner(argv, limit, timeout):
                seen.append((argv[0], os.environ.get("GH_TOKEN"), os.environ.get("ACTIONS_RUNTIME_TOKEN")))
                raise subprocess.CalledProcessError(2, argv)

            for label in ("derive-independent-expectations", "acquire-producer"):
                with self.assertRaises(gate.GateError):
                    gate.run_command(work, runner, label, [label])
                self.assertEqual(os.environ.get("GH_TOKEN"), "secret")
                self.assertEqual(os.environ.get("ACTIONS_RUNTIME_TOKEN"), "runtime")
            self.assertEqual(seen, [("derive-independent-expectations", None, None),
                                    ("acquire-producer", "secret", None)])
            self.assertEqual(os.environ.get("GH_TOKEN"), "secret")
            self.assertEqual(os.environ.get("ACTIONS_RUNTIME_TOKEN"), "runtime")

    def test_pipeline_deadline_rejects_before_child_and_caps_step_timeout(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp).resolve()
            (work / "logs").mkdir()
            with self.assertRaisesRegex(gate.GateError, "pipeline_deadline_exceeded"):
                gate.run_command(work, mock.Mock(), "expired", ["x"], deadline=gate.time.monotonic() - 1)
            runner = mock.Mock(return_value=(0, b"ok"))
            gate.run_command(work, runner, "bounded", ["x"], timeout=60,
                             deadline=gate.time.monotonic() + 2.2)
            self.assertEqual(runner.call_args.args[2], 2)

    def test_pipeline_deadline_rejects_child_that_returns_after_budget(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp).resolve()
            (work / "logs").mkdir()
            with mock.patch.object(gate.time, "monotonic", side_effect=[10.0, 12.0, 12.1]):
                with self.assertRaisesRegex(gate.GateError, "pipeline_deadline_exceeded"):
                    gate.run_command(work, mock.Mock(return_value=(0, b"ok")), "late", ["x"],
                                     timeout=20, deadline=11.0)

    def test_captured_script_bytes_are_checked_before_and_after_execution(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            commit, _ = self._git_fixture(root)
            work = root / "work"
            captured = work / "captured-tools/fixture.txt"
            captured.parent.mkdir(parents=True)
            captured.write_bytes(b"tracked\n")
            (work / "logs").mkdir()
            def runner(argv, limit, timeout):
                captured.write_bytes(b"changed during child\n")
                return 0, b"ok"
            with self.assertRaisesRegex(gate.GateError, "captured_script_changed_during_execution"):
                gate.run_trusted_script(root, commit, {"fixture.txt": b"tracked\n"}, work,
                    runner, "fixture", "fixture.txt", ["python", "fixture.py"])

    def test_pin_hash_can_be_taken_from_exact_bytes_used_for_parse(self) -> None:
        payload = json_bytes({"run_id": "1", "artifact_id": "2", "source_commit": COMMIT,
                              "producer_tree": "c" * 40, "archive_zip_sha256": SHA, "archive_zip_bytes": "10"})
        parsed = gate.validate_pin_bytes(payload, gate.PRODUCER_PIN_KEYS, "producer_pins")
        self.assertEqual(parsed["run_id"], "1")
        self.assertEqual(digest(payload), hashlib.sha256(payload).hexdigest())

    def test_main_failure_does_not_write_into_preexisting_work_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            work = root / "work"
            work.mkdir()
            marker = work / "owner-marker"
            marker.write_text("preserve")
            code = gate.main(["--repository", str(root), "--release-source-commit", "bad",
                "--producer-pins-json", str(root / "missing-producer.json"),
                "--consumer-pins-json", str(root / "missing-consumer.json"),
                "--work-dir", str(work), "--output", str(work / gate.RELEASE_OUTPUT_NAME)])
            self.assertEqual(code, 1)
            self.assertEqual(sorted(path.name for path in work.iterdir()), ["owner-marker"])

    def test_actual_archive_readback_report_is_exact_source_index_and_scope_bound(self) -> None:
        report = {"schema": "kairos-actual-archive-release-readback-v1", "result": "pass",
                  "release_stage": "actual-package-archives", "source_commit": COMMIT,
                  "archive_index_sha256": SHA, "archive_count": 8, "ecosystem_count": 7,
                  "copied_archive_bytes": 1234, "claim_scope": gate.READBACK_CLAIM_SCOPE}
        gate.validate_readback_report(report, SHA, COMMIT)
        for key, bad in (("result", "fail"), ("source_commit", "d" * 40),
                         ("archive_index_sha256", "e" * 64), ("archive_count", 7),
                         ("copied_archive_bytes", True), ("claim_scope", "complete release accepted")):
            with self.subTest(key=key), self.assertRaises(gate.GateError):
                gate.validate_readback_report({**report, key: bad}, SHA, COMMIT)
        with self.assertRaises(gate.GateError):
            gate.validate_readback_report({**report, "unexpected": True}, SHA, COMMIT)

    def test_explicit_pin_json_requires_exact_all_string_fields(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            value = {"run_id": "1", "artifact_id": "2", "source_commit": COMMIT,
                     "producer_tree": "c" * 40, "archive_zip_sha256": SHA, "archive_zip_bytes": "10"}
            path = root / "producer.json"
            path.write_bytes(json_bytes(value))
            self.assertEqual(gate.validate_pin_file(path, gate.PRODUCER_PIN_KEYS, "producer_pins"), value)
            for changed in (
                {key: val for key, val in value.items() if key != "artifact_id"},
                {**value, "extra": "unexpected"},
                {**value, "run_id": 1},
            ):
                path.write_bytes(json_bytes(changed))
                with self.assertRaises(gate.GateError):
                    gate.validate_pin_file(path, gate.PRODUCER_PIN_KEYS, "producer_pins")

    def test_pin_json_rejects_duplicate_nonfinite_overflow_truncated_and_oversize(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp).resolve() / "pins.json"
            payloads = [b'{"run_id":"1","run_id":"2"}', b'{"n":NaN}', b'{"n":1e999}', b'{',
                        b" " * (gate.MAX_PIN_BYTES + 1)]
            for payload in payloads:
                path.write_bytes(payload)
                with self.subTest(length=len(payload)):
                    with self.assertRaises(gate.GateError):
                        gate.validate_pin_file(path, gate.PRODUCER_PIN_KEYS, "producer_pins")

    def test_decimal_ids_and_bytes_are_bounded_and_canonical(self) -> None:
        for value in ("0", "01", "+1", "-1", "1.0", "1e2", "", "9" * 20):
            with self.subTest(value=value):
                with self.assertRaises(gate.GateError):
                    gate.positive_pin(value, "pin", maximum=100)
        self.assertEqual(gate.positive_pin("100", "pin", maximum=100), 100)

    def test_release_source_and_producer_pin_mismatch_reject_before_api_or_workdir(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            pins1, pins2 = root / "producer.json", root / "consumer.json"
            pins1.write_bytes(json_bytes({"run_id": "1", "artifact_id": "2", "source_commit": "c" * 40,
                "producer_tree": "d" * 40, "archive_zip_sha256": SHA, "archive_zip_bytes": "12"}))
            pins2.write_bytes(json_bytes({"run_id": "3", "run_attempt": "1", "artifact_id": "4",
                "source_commit": COMMIT, "archive_zip_sha256": "e" * 64, "archive_zip_bytes": "100"}))
            work = root / "work"
            args = argparse.Namespace(repository=root, release_source_commit=COMMIT,
                                      producer_pins_json=pins1, consumer_pins_json=pins2,
                                      work_dir=work, output=work / gate.RELEASE_OUTPUT_NAME)
            with mock.patch.object(gate.platform, "system", return_value="Linux"), \
                 mock.patch.object(gate.platform, "machine", return_value="x86_64"), \
                 mock.patch.object(gate, "run_command") as command:
                with self.assertRaisesRegex(gate.GateError, "release_source_pin_mismatch"):
                    gate.prepare(args)
            command.assert_not_called()
            self.assertFalse(work.exists())

    def _git_fixture(self, root: Path) -> tuple[str, str]:
        subprocess.run(["git", "init", "-q", str(root)], check=True)
        subprocess.run(["git", "-C", str(root), "config", "user.name", "Test"], check=True)
        subprocess.run(["git", "-C", str(root), "config", "user.email", "test@example.invalid"], check=True)
        (root / "fixture.txt").write_text("tracked\n")
        subprocess.run(["git", "-C", str(root), "add", "fixture.txt"], check=True)
        subprocess.run(["git", "-C", str(root), "commit", "-qm", "fixture"], check=True)
        commit = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"], check=True,
                                capture_output=True, text=True).stdout.strip()
        tree = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD^{tree}"], check=True,
                              capture_output=True, text=True).stdout.strip()
        return commit, tree

    def test_release_tree_pin_mismatch_rejects_before_any_acquisition(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve() / "repo"
            root.mkdir()
            commit, tree = self._git_fixture(root)
            pins1, pins2 = root.parent / "producer.json", root.parent / "consumer.json"
            pins1.write_bytes(json_bytes({"run_id": "1", "artifact_id": "2", "source_commit": commit,
                "producer_tree": "f" * 40, "archive_zip_sha256": SHA, "archive_zip_bytes": "12"}))
            pins2.write_bytes(json_bytes({"run_id": "3", "run_attempt": "1", "artifact_id": "4",
                "source_commit": commit, "archive_zip_sha256": "e" * 64, "archive_zip_bytes": "100"}))
            work = root.parent / "work"
            args = argparse.Namespace(repository=root, release_source_commit=commit,
                                      producer_pins_json=pins1, consumer_pins_json=pins2,
                                      work_dir=work, output=work / gate.RELEASE_OUTPUT_NAME)
            with mock.patch.object(gate.platform, "system", return_value="Linux"), \
                 mock.patch.object(gate.platform, "machine", return_value="x86_64"), \
                 mock.patch.object(gate, "run_command") as command:
                with self.assertRaisesRegex(gate.GateError, "producer_tree_pin_mismatch"):
                    gate.prepare(args)
            command.assert_not_called()
            self.assertEqual(gate.git_text(root, "rev-parse", commit + "^{tree}"), tree)
            self.assertFalse(work.exists())

    def test_trusted_source_rejects_uncommitted_helper_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            commit, _ = self._git_fixture(root)
            (root / "fixture.txt").write_text("changed\n")
            with self.assertRaisesRegex(gate.GateError, "checkout_source_differs_from_trusted_git"):
                gate.trusted_source(root, commit, "fixture.txt")

    def _syft_fixture(self, root: Path, label: str, *, original: bool) -> tuple[Path, Path, dict, dict, bytes]:
        base = root / label
        (base / "evidence").mkdir(parents=True)
        (base / "logs").mkdir()
        binary_sha = digest(b"trusted-linux-syft")
        output = f"/home/runner/work/_temp/{label}/syft-linux-amd64-1-1"
        python = "/opt/hostedtoolcache/Python/3.14.8/x64/bin/python3"
        installer = "/home/runner/work/kairos/kairos/scripts/supply_chain/install_verified_syft.py"
        checksum = output + "/downloads/syft-checksums.txt"
        bundle = output + "/downloads/syft-checksums.sigstore.json"
        archive_path = output + "/downloads/syft_1.54.0_linux_amd64.tar.gz"
        commands = [
            [python, installer, "--fetch-internal", gate.SYFT_CHECKSUM_URL, checksum, "65536"],
            [python, installer, "--fetch-internal", gate.SYFT_BUNDLE_URL, bundle, "2097152"],
            [python, "-m", "venv", output + "/verifier"],
            [output + "/verifier/bin/python", output + "/pip-config-audit.py", output + "/pip-canary.ini"],
            [output + "/verifier/bin/python", "-m", "pip", "--isolated", "--disable-pip-version-check",
             "--no-input", "install", "--require-hashes", "-r", output + "/verifier.lock"],
            [output + "/verifier/bin/python", "-m", "sigstore", "verify", "github", "--bundle", bundle,
             "--cert-identity", gate.SYFT_CERT_IDENTITY, "--sha", gate.SYFT_RELEASE_COMMIT,
             "--repository", "anchore/syft", "--ref", "refs/heads/main", checksum],
            [python, installer, "--fetch-internal", gate.SYFT_ARCHIVE_URL, archive_path, str(64 * 1024 * 1024)],
            [python, installer, "--extract-internal", archive_path, output + "/extract", "2" * 64],
            [output + "/bin/syft", "version", "-o", "json"],
        ]
        stable = {
            "schema": "kairos.verified-syft-installer.v1", "result": "pass", "version": "1.54.0",
            "release_tag": "v1.54.0", "release_commit": "d" * 40, "target": "linux-amd64",
            "platform": "linux/amd64", "release_checksum_sha256": "e" * 64,
            "release_bundle_sha256": "f" * 64, "authenticated_asset": "syft_1.54.0_linux_amd64.tar.gz",
            "signed_asset_sha256": "1" * 64, "archive_sha256": "2" * 64,
            "binary_path": "bin/syft",
            "binary_sha256": binary_sha,
            "archive": {"member_count": 4, "binary_sha256": binary_sha,
                        "archive_sha256": "2" * 64,
                        "members": {"LICENSE": "a" * 64, "README.md": "b" * 64,
                                    "syft": binary_sha, "CHANGELOG.md": "c" * 64}},
            "version_probe": {"application": "syft", "version": "1.54.0", "platform": "linux/amd64",
                               "gitCommit": "d" * 40},
            "issuer_policy": "issuer", "issuer_enforcement": "policy", "certificate_identity": "identity",
            "repository": "anchore/syft", "ref": "refs/tags/v1.54.0", "installer_source_sha256": "4" * 64,
            "verifier": {"sigstore": "4.5.0", "lock_path": "syft-linux-verifier.lock", "lock_sha256": "5" * 64},
            "qualification_limit": "Native authenticated installation and version probe only; no package scan, release, or publication is represented.",
        }
        command_rows = []
        for index, log_label in enumerate(gate.SYFT_COMMAND_LABELS):
            if log_label == "verify-signed-checksum-document":
                data = f"OK: {checksum}\n".encode()
            elif log_label == "extract-syft-archive":
                data = json_bytes({"archive": stable["archive"], "binary": "syft"})
            elif log_label == "syft-version":
                data = json_bytes(stable["version_probe"])
            else:
                data = f"log-{label}-{index}\n".encode()
            (base / "logs" / f"{index:02d}-{log_label}.log").write_bytes(data)
            command_rows.append({"label": log_label, "exit_status": 0, "log_sha256": digest(data), "log_bytes": len(data),
                                 "stdout_sha256": digest(data), "stderr_sha256": digest(b""),
                                 "argv": commands[index]})
        command_rows[5]["stdout_sha256"] = digest(b"")
        command_rows[5]["stderr_sha256"] = digest((base / "logs/05-verify-signed-checksum-document.log").read_bytes())
        receipt = {**stable, "commands": command_rows, "events": [],
                   "python_toolchain": {"version": "3.14.8 (native fixture)",
                                        "executable": "/opt/hostedtoolcache/Python/3.14.8/x64/bin/python3.14"}}
        receipt_path = base / "evidence/receipt.json"
        receipt_bytes = json_bytes(receipt)
        receipt_path.write_bytes(receipt_bytes)
        report = {"schema": "kairos.syft-installation-validation.v1", "result": "pass",
                  "target": "linux-amd64", "platform": "linux/amd64", "version": "1.54.0",
                  "installer_source_sha256": "4" * 64, "verifier_lock_sha256": "5" * 64,
                  "receipt_sha256": digest(receipt_bytes), "archive_sha256": "2" * 64,
                  "binary_sha256": binary_sha, "validated_commands": 9, "validated_logs": 9,
                  "validated_members": ["LICENSE", "README.md", "syft", "syft.sig"]}
        report_path = base / "evidence/validation-report.json"
        report_path.write_bytes(json_bytes(report))
        (base / "bin").mkdir()
        (base / "bin/syft").write_bytes(b"trusted-linux-syft")
        return base, report_path, receipt, report, receipt_bytes

    def test_native_retained_syft_receipt_and_logs_are_self_contained_and_accepted(self) -> None:
        canonical = zlib.decompress(base64.b64decode(NATIVE_SYFT_FIXTURE_ZLIB_B64))
        self.assertEqual(digest(canonical), NATIVE_SYFT_FIXTURE_SHA256)
        native = json.loads(canonical)
        receipt = {key: native[key] for key in ("commands", "archive", "version_probe", "python_toolchain")}
        self.assertEqual(receipt["commands"][0]["argv"][0],
                         "/opt/hostedtoolcache/Python/3.14.8/x64/bin/python3")
        self.assertEqual(receipt["python_toolchain"]["executable"],
                         "/opt/hostedtoolcache/Python/3.14.8/x64/bin/python3.14")
        with tempfile.TemporaryDirectory() as temp:
            original = Path(temp).resolve()
            (original / "logs").mkdir()
            for name, content in native["logs"].items():
                (original / "logs" / name).write_bytes(content.encode("utf-8"))
            rows = gate.validate_original_syft_commands(original, receipt, run_id=37355352626, attempt=1)
            self.assertEqual(len(rows), 9)
            self.assertEqual([row["sha256"] for row in rows],
                             [record["log_sha256"] for record in receipt["commands"]])
            def replace_argv(value, indices, position, replacement):
                for index in indices:
                    value["commands"][index]["argv"][position] = replacement

            for mutate in (
                lambda value: replace_argv(value, (0, 1, 2, 6, 7), 0, "/opt/hostedtoolcache/Python/3.14.8/x64/bin/python-bogus"),
                lambda value: replace_argv(value, (0, 1, 2, 6, 7), 0, "python3"),
                lambda value: value["commands"][1]["argv"].__setitem__(0, "/usr/bin/python3"),
                lambda value: value["python_toolchain"].__setitem__("executable", "/usr/bin/python3"),
                lambda value: value["python_toolchain"].__setitem__("version", "3.13.0"),
                lambda value: replace_argv(value, (0, 1, 6, 7), 1, "/tmp/../../scripts/supply_chain/install_verified_syft.py"),
                lambda value: replace_argv(value, (0, 1, 6, 7), 1, "/tmp/\x00/scripts/supply_chain/install_verified_syft.py"),
                lambda value: replace_argv(value, (0, 1, 6, 7), 1, "/tmp/other-installer.py"),
            ):
                changed = copy.deepcopy(receipt)
                mutate(changed)
                with self.subTest(command=changed["commands"][0]["argv"][:2]), self.assertRaises(gate.GateError):
                    gate.validate_original_syft_commands(original, changed, run_id=37355352626, attempt=1)

    def test_original_and_fresh_syft_receipt_identity_allows_context_hash_difference(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            original, original_report_path, old_receipt, old_report, old_bytes = self._syft_fixture(root, "old", original=True)
            fresh, _, fresh_receipt, fresh_report, fresh_bytes = self._syft_fixture(root, "fresh", original=False)
            prep = {"syft_qualification": {"receipt_sha256": digest(old_bytes), "binary_sha256": digest(b"trusted-linux-syft"),
                                           "target": "linux-amd64", "platform": "linux/amd64",
                                           "installer_source_sha256": "4" * 64, "verifier_lock_sha256": "5" * 64}}
            log_reads = []
            original_read = gate.secure_read
            def tracked_read(path, limit, label):
                if label == "original_syft_log":
                    log_reads.append(Path(path))
                return original_read(path, limit, label)
            with mock.patch.object(gate, "secure_read", side_effect=tracked_read):
                binary, receipt_path, binary_sha = gate.validate_syft_receipt(
                    original, original_report_path, fresh, fresh_report, prep, root,
                    consumer_run=1, consumer_attempt=1)
            self.assertNotEqual(digest(old_bytes), digest(fresh_bytes))
            self.assertEqual(binary_sha, digest(b"trusted-linux-syft"))
            self.assertTrue(binary.is_file())
            self.assertTrue(receipt_path.is_file())
            self.assertEqual(len(log_reads), 9)
            self.assertEqual(len(set(log_reads)), 9)
            record = json.loads((root / "syft-requalification.json").read_bytes())
            self.assertTrue(record["receipt_sha_difference_expected"])
            self.assertEqual(len(record["original_validated_log_hashes"]), 9)

    def test_original_syft_log_tamper_rejects_requalification(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            original, report_path, old_receipt, old_report, old_bytes = self._syft_fixture(root, "old", original=True)
            fresh, _, _, fresh_report, _ = self._syft_fixture(root, "fresh", original=False)
            prep = {"syft_qualification": {"receipt_sha256": digest(old_bytes), "binary_sha256": digest(b"trusted-linux-syft"),
                                           "target": "linux-amd64", "platform": "linux/amd64",
                                           "installer_source_sha256": "4" * 64, "verifier_lock_sha256": "5" * 64}}
            (original / "logs/00-download-checksums.log").write_text("tampered\n")
            with self.assertRaisesRegex(gate.GateError, "original_syft_log_hash_mismatch"):
                gate.validate_syft_receipt(original, report_path, fresh, fresh_report, prep, root,
                                           consumer_run=1, consumer_attempt=1)

    def test_syft_identity_change_rejects_even_if_receipt_hash_is_independent(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            original, report_path, old_receipt, old_report, old_bytes = self._syft_fixture(root, "old", original=True)
            fresh, _, new_receipt, fresh_report, _ = self._syft_fixture(root, "fresh", original=False)
            new_receipt["release_commit"] = "9" * 40
            new_bytes = json_bytes(new_receipt)
            (fresh / "evidence/receipt.json").write_bytes(new_bytes)
            fresh_report["receipt_sha256"] = digest(new_bytes)
            prep = {"syft_qualification": {"receipt_sha256": digest(old_bytes), "binary_sha256": digest(b"trusted-linux-syft"),
                                           "target": "linux-amd64", "platform": "linux/amd64",
                                           "installer_source_sha256": "4" * 64, "verifier_lock_sha256": "5" * 64}}
            with self.assertRaisesRegex(gate.GateError, "original_and_fresh_syft_identity_mismatch"):
                gate.validate_syft_receipt(original, report_path, fresh, fresh_report, prep, root,
                                           consumer_run=1, consumer_attempt=1)

    def test_missing_or_modified_embedded_expectation_file_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            work = root / "work"
            work.mkdir()
            expectations = root / "expectations"
            expectations.mkdir()
            values = {
                "acquisition.json": {"source_commit": COMMIT},
                "expected-inputs.json": {"source_commit": COMMIT, "original_run_id": 1,
                                         "acquisition_artifact_id": 2},
                "outer-binding.json": {"source_commit": COMMIT, "producer_tree": "c" * 40,
                                       "archive_zip_sha256": SHA, "archive_zip_bytes": 50},
            }
            for name, value in values.items():
                (expectations / name).write_bytes(json_bytes(value))
            prep = {"trusted_consumer_sha": COMMIT,
                    "producer_pins": {"repository": gate.REPOSITORY, "run_id": 1, "artifact_id": 2,
                        "source_commit": COMMIT, "producer_tree": "c" * 40,
                        "archive_zip_sha256": SHA, "archive_zip_bytes": 50},
                    "prepared_files": {name: digest(json_bytes(value)) for name, value in values.items()}}
            producer = {"run_id": 1, "artifact_id": 2, "producer_tree": "c" * 40,
                        "archive_zip_sha256": SHA, "archive_zip_bytes": 50}
            gate.validate_embedded_preparation(expectations.parent, prep, work, source=COMMIT,
                                               producer_values=producer)
            (expectations / "outer-binding.json").unlink()
            with self.assertRaises(gate.GateError):
                gate.validate_embedded_preparation(expectations.parent, prep, work, source=COMMIT,
                                                   producer_values=producer)

    def test_failed_helper_process_is_recorded_and_stops_pipeline(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            work = Path(temp).resolve()
            (work / "logs").mkdir()

            def failed(argv, limit, timeout):
                raise subprocess.CalledProcessError(23, argv)

            with self.assertRaisesRegex(gate.GateError, "command_acquire-producer_failed"):
                gate.run_command(work, failed, "acquire-producer", ["python", "script.py"])
            record = json.loads((work / "command-records.json").read_bytes())[0]
            self.assertEqual(record["exit_status"], 23)
            self.assertEqual(record["stdout_sha256"], digest(b""))

    def test_changed_helper_source_rejects_before_child_invocation(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            commit, _ = self._git_fixture(root)
            (root / "fixture.txt").write_text("different from trusted blob\n")
            work = root / "work"
            (work / "captured-tools").mkdir(parents=True)
            (work / "captured-tools/fixture.txt").write_bytes(b"tracked\n")
            with mock.patch.object(gate, "run_command") as command:
                with self.assertRaisesRegex(gate.GateError, "checkout_source_differs_from_trusted_git"):
                    gate.run_trusted_script(root, commit, {"fixture.txt": b"tracked\n"}, work,
                        None, "fixture", "fixture.txt", ["python", "fixture.py"])
            command.assert_not_called()

    def test_captured_imported_schema_change_rejects_after_child(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.name", "Test"], check=True)
            subprocess.run(["git", "-C", str(root), "config", "user.email", "test@example.invalid"], check=True)
            (root / "script.py").write_bytes(b"script source\n")
            (root / "schema.json").write_bytes(b"{\"version\":1}\n")
            subprocess.run(["git", "-C", str(root), "add", "script.py", "schema.json"], check=True)
            subprocess.run(["git", "-C", str(root), "commit", "-qm", "fixture"], check=True)
            commit = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"], check=True,
                                    capture_output=True, text=True).stdout.strip()
            work = root / "work"
            (work / "captured-tools").mkdir(parents=True)
            (work / "logs").mkdir()
            captured_script = work / "captured-tools/script.py"
            captured_schema = work / "captured-tools/schema.json"
            captured_script.write_bytes(b"script source\n")
            captured_schema.write_bytes(b"{\"version\":1}\n")
            def runner(argv, limit, timeout):
                captured_schema.write_bytes(b"{\"version\":2}\n")
                return 0, b"ok"
            sources = {"script.py": b"script source\n", "schema.json": b"{\"version\":1}\n"}
            with self.assertRaisesRegex(gate.GateError, "captured_script_changed_during_execution"):
                gate.run_trusted_script(root, commit, sources, work, runner, "fixture",
                                        "script.py", ["python", str(captured_script)])

    def test_full_archive_report_requires_exact_qualified_profile_counts(self) -> None:
        value = {"valid": True, "profile": "kairos-archive-copy-evidence-v1", "archive_count": 8,
                 "ecosystem_count": 7, "spdx_document_count": 9, "evidence_file_count": 44,
                 "archive_index_sha256": SHA, "statement_sha256": "c" * 64,
                 "claim_scope": gate.CLAIM_SCOPE}
        gate.validate_report(value, SHA)
        for key, bad in (("valid", False), ("archive_count", 7), ("ecosystem_count", 6),
                         ("spdx_document_count", 8), ("evidence_file_count", 43),
                         ("archive_index_sha256", "d" * 64), ("claim_scope", "release accepted")):
            changed = {**value, key: bad}
            with self.subTest(key=key), self.assertRaises(gate.GateError):
                gate.validate_report(changed, SHA)

    def test_command_sequence_has_no_package_build_or_scanner_step(self) -> None:
        source = Path(gate.__file__).read_text()
        tree = __import__("ast").parse(source)
        prepare_node = next(node for node in tree.body if isinstance(node, __import__("ast").FunctionDef)
                            and node.name == "prepare")
        labels = []
        calls = [node for node in __import__("ast").walk(prepare_node)
                 if isinstance(node, __import__("ast").Call) and isinstance(node.func, __import__("ast").Name)
                 and node.func.id == "run_trusted_script"]
        for node in sorted(calls, key=lambda item: item.lineno):
            if isinstance(node, __import__("ast").Call) and isinstance(node.func, __import__("ast").Name) \
                    and node.func.id == "run_trusted_script" and len(node.args) > 5:
                if isinstance(node.args[5], __import__("ast").Constant):
                    labels.append(node.args[5].value)
        self.assertEqual(labels, ["acquire-producer", "acquire-consumer",
            "install-fresh-syft", "validate-fresh-syft", "derive-independent-expectations",
            "verify-full-archive-profile", "prepare-verified-archive-output"])
        self.assertNotIn("cargo", source)
        invoked_scripts = [node.args[6].value for node in calls if len(node.args) > 6
                           and isinstance(node.args[6], __import__("ast").Constant)]
        self.assertNotIn("packaging/scripts/build_archive_supply_chain.py", invoked_scripts)


if __name__ == "__main__":
    unittest.main()

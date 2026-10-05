from __future__ import annotations

import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

SCRIPT = Path(__file__).resolve().parents[1] / "packaging/scripts/build_archive_evidence_expectations.py"
SPEC = importlib.util.spec_from_file_location("archive_expectations_builder_test", SCRIPT)
assert SPEC and SPEC.loader
builder = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = builder
SPEC.loader.exec_module(builder)

FIXTURE_SPEC = importlib.util.spec_from_file_location(
    "archive_expectations_acquisition_fixture",
    Path(__file__).with_name("test_archive_supply_chain_evidence_verifier.py"),
)
assert FIXTURE_SPEC and FIXTURE_SPEC.loader
fixture_module = importlib.util.module_from_spec(FIXTURE_SPEC)
sys.modules[FIXTURE_SPEC.name] = fixture_module
FIXTURE_SPEC.loader.exec_module(fixture_module)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class ExpectationFixture:
    def __init__(self, root: Path, *, main_head_sha: str | None = None):
        self.root = root.resolve()
        self.acquisition = self.root / "acquisition"
        self.acq = fixture_module.CompleteEvidenceFixture(self.root, main_dispatch=True, main_head_sha=main_head_sha)
        retained_zip = self.acquisition / f"{self.acq.artifact_id}.zip"
        self.acq.archive_zip.rename(retained_zip)
        self.acq.archive_zip = retained_zip
        self.bundle = self.acquisition / "bundle"
        self.consumer = self.root / "consumer"
        self.consumer.mkdir()
        self.binary = self.root / "syft"
        self.binary.write_bytes(b"synthetic fixture binary; never executed")
        synthetic_binary_sha = digest(self.binary.read_bytes())
        source_root = SCRIPT.parents[2]
        trusted_paths = (*builder.HELPERS, builder.VERIFIER, builder.BUILDER, builder.SCHEMA,
                         builder.SYFT_INSTALLER, builder.SYFT_DARWIN_LOCK, builder.SYFT_LINUX_LOCK)
        for relative in trusted_paths:
            destination = self.consumer / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            data = (source_root / relative).read_bytes()
            if relative == builder.SYFT_INSTALLER:
                data = data.replace(b"835607cdfbdbfc59335b0beadeefc47aa6aab7d3b403c11cfa65627d92a27f61", synthetic_binary_sha.encode())
                data = data.replace(b"d46a9a61a6ae3d367f0a03748c5e9c59253e586c4388ab26ddcacebc2efa0d92", synthetic_binary_sha.encode())
            destination.write_bytes(data)
        subprocess.run(["git", "init", "-q"], cwd=self.consumer, check=True, timeout=builder.GIT_TIMEOUT)
        subprocess.run(["git", "add", *trusted_paths], cwd=self.consumer, check=True, timeout=builder.GIT_TIMEOUT)
        subprocess.run(["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "trusted consumer fixture"],
                       cwd=self.consumer, check=True, timeout=builder.GIT_TIMEOUT)
        self.consumer_sha = subprocess.run(
            ["git", "rev-parse", "--verify", "HEAD"], cwd=self.consumer,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=builder.GIT_TIMEOUT,
            check=True, text=True,
        ).stdout.strip()
        self.receipt = self.root / "qualified-syft-receipt.json"
        self.set_target_receipt("Darwin", "arm64")
        self.output = self.root / "derived"
        self.preparation_receipt = self.root / "preparation-receipt.json"

    def set_target_receipt(self, system: str, machine: str) -> dict[str, object]:
        installer_blob = subprocess.run(
            ["git", "show", f"HEAD:{builder.SYFT_INSTALLER}"], cwd=self.consumer,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=builder.GIT_TIMEOUT,
            check=True,
        ).stdout
        installer = builder.load_trusted_module(
            None, installer_blob, "archive_expectations_fixture_installer", builder.SYFT_INSTALLER,
        )
        target = installer.detect_target(system, machine)
        lock_path = self.consumer / "scripts/supply_chain" / target["verifier_lock"]
        source_sha = digest((self.consumer / builder.SYFT_INSTALLER).read_bytes())
        lock_sha = digest(lock_path.read_bytes())
        release_commit = installer.RELEASE_COMMIT
        receipt = {
            "schema": "kairos.verified-syft-installer.v1",
            "result": "pass",
            "repository": installer.REPOSITORY,
            "ref": installer.WORKFLOW_REF,
            "target": target["key"],
            "platform": target["platform"],
            "version": installer.VERSION,
            "release_commit": release_commit,
            "binary_sha256": digest(self.binary.read_bytes()),
            "installer_source_sha256": source_sha,
            "verifier": {"lock_path": target["verifier_lock"], "lock_sha256": lock_sha},
            "version_probe": {
                "application": "syft", "version": installer.VERSION,
                "platform": target["platform"], "gitCommit": release_commit,
            },
            "qualification_limit": builder.SYFT_QUALIFICATION_LIMIT,
        }
        self.receipt.write_bytes(builder.canonical(receipt))
        return target

    def argv(self, **changes: object) -> list[str]:
        values: dict[str, object] = {
            "--repository": self.consumer,
            "--trusted-consumer-sha": self.consumer_sha,
            "--acquisition-dir": self.acquisition,
            "--archive-bundle": self.bundle,
            "--archive-zip": self.acq.archive_zip,
            "--run-id": self.acq.run_id,
            "--artifact-id": self.acq.artifact_id,
            "--source-commit": self.acq.commit,
            "--producer-tree": self.acq.tree,
            "--archive-zip-sha256": digest(self.acq.archive_zip_bytes),
            "--archive-zip-bytes": len(self.acq.archive_zip_bytes),
            "--syft": self.binary,
            "--syft-sha256": digest(self.binary.read_bytes()),
            "--syft-receipt": self.receipt,
            "--syft-receipt-sha256": digest(self.receipt.read_bytes()),
            "--output-dir": self.output,
            "--preparation-receipt": self.preparation_receipt,
        }
        values.update(changes)
        result: list[str] = []
        for key, value in values.items():
            result.extend((key, str(value)))
        return result


class ArchiveExpectationTests(unittest.TestCase):
    def run_main(self, fixture: ExpectationFixture, argv: list[str], host: tuple[str, str] = ("Darwin", "arm64")) -> tuple[int, str, str]:
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr), \
             mock.patch.object(builder.platform, "system", return_value=host[0]), \
             mock.patch.object(builder.platform, "machine", return_value=host[1]):
            try:
                result = builder.main(argv)
            except SystemExit as exc:
                result = int(exc.code)
        return result, stdout.getvalue(), stderr.getvalue()

    def test_derives_exact_binding_map_and_adapter_before_evidence_exists(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            self.assertFalse(fixture.output.exists())
            code, stdout, _ = self.run_main(fixture, fixture.argv())
            self.assertEqual(code, 0, stdout)
            self.assertFalse((fixture.output / "provenance.json").exists())
            self.assertEqual({path.name for path in fixture.output.iterdir()}, {
                "outer-binding.json", "expected-inputs.json", "acquisition.json",
            })
            prep = json.loads(fixture.preparation_receipt.read_bytes())
            self.assertEqual(prep["trusted_consumer_sha"], fixture.consumer_sha)
            self.assertEqual(prep["syft_qualification"]["receipt_path"], str(fixture.receipt.absolute()))
            self.assertEqual(prep["syft_qualification"]["receipt_sha256"], digest(fixture.receipt.read_bytes()))
            self.assertEqual(prep["syft_qualification"]["platform"], "darwin/arm64")
            self.assertEqual(prep["syft_qualification"]["verifier_lock_path"], builder.SYFT_DARWIN_LOCK)
            self.assertEqual(
                prep["syft_qualification"]["verifier_lock_sha256"],
                digest((fixture.consumer / builder.SYFT_DARWIN_LOCK).read_bytes()),
            )
            self.assertEqual(set(prep["prepared_files"]), {"outer-binding.json", "expected-inputs.json", "acquisition.json"})
            binding = json.loads((fixture.output / "outer-binding.json").read_bytes())
            expected = json.loads((fixture.output / "expected-inputs.json").read_bytes())
            adapter = json.loads((fixture.output / "acquisition.json").read_bytes())
            self.assertEqual(set(binding), set(builder.load_trusted_module(
                None, builder.trusted_blobs(fixture.consumer, fixture.consumer_sha)[0][builder.VERIFIER],
                "test_verifier", builder.VERIFIER,
            ).EXPECTED_BINDING_FIELDS))
            self.assertEqual(set(expected), {"archive_index_sha256", "source_commit", "original_run_id", "acquisition_artifact_id", "dependencies"})
            self.assertEqual(len(expected["dependencies"]), 12)
            expected_dependency_order = [
                f"https://github.com/edithatogo/kairos/actions/runs/{fixture.acq.run_id}",
                "ARCHIVE-INDEX.json",
                "build-inputs/ARCHIVE-INDEX.json",
                "build-inputs/BUILD-RECEIPT.json",
                "build-inputs/acquisition.json",
                "packaging/scripts/build_archive_supply_chain.py",
                "packaging/scripts/build_archive_release_manifest.py",
                "packaging/scripts/build_package_archive_bundle.py",
                "packaging/scripts/acquire_package_archive_bundle.py",
                "packaging/scripts/validate_archive_copy_provenance.py",
                "tool:syft",
                "schema:spdx-2.3",
            ]
            self.assertEqual([item["id"] for item in expected["dependencies"]], expected_dependency_order)
            self.assertEqual(set(adapter), {"archive_count", "archive_index_sha256", "artifact_digest", "artifact_id", "derivation", "ecosystems", "repository", "run_id", "source_commit"})
            self.assertEqual(adapter["archive_count"], 7)
            self.assertEqual(adapter["derivation"]["status"], "derived local adapter receipt; not original acquisition history")

    def test_rejects_wrong_trusted_commit_and_dirty_helper_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            code, _, _ = self.run_main(fixture, fixture.argv(**{"--trusted-consumer-sha": "0" * 40}))
            self.assertEqual(code, 1)
            original = builder.read_nofollow

            def dirty(path: Path, limit: int, label: str) -> bytes:
                data = original(path, limit, label)
                if label == builder.HELPERS[0]:
                    return data + b"\n# modified after trusted commit\n"
                return data

            with mock.patch.object(builder, "read_nofollow", side_effect=dirty):
                code, _, _ = self.run_main(fixture, fixture.argv())
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())

    def test_rejects_missing_main_raw_readback(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            (fixture.acquisition / "source-commit-api-readback.json").unlink()
            code, _, _ = self.run_main(fixture, fixture.argv())
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())

    def test_main_advanced_requires_native_compare_and_binds_its_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary), main_head_sha="e" * 40)
            code, stdout, _ = self.run_main(fixture, fixture.argv())
            self.assertEqual(code, 0, stdout)
            adapter = json.loads((fixture.output / "acquisition.json").read_bytes())
            inputs = adapter["derivation"]["inputs"]
            compare = fixture.acquisition / "compare-main-readback.json"
            self.assertEqual(inputs["compare_main_readback_json_sha256"], digest(compare.read_bytes()))
            self.assertEqual(inputs["branch_main_readback_json_sha256"], digest((fixture.acquisition / "branch-main-readback.json").read_bytes()))

    def test_rejects_unqualified_syft_pins_and_platform_claims(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            code, _, _ = self.run_main(fixture, fixture.argv(**{"--syft-sha256": "0" * 64}))
            self.assertEqual(code, 1)
            bad = json.loads(fixture.receipt.read_bytes())
            bad["verifier"]["lock_sha256"] = "0" * 64
            fixture.receipt.write_bytes(builder.canonical(bad))
            code, _, _ = self.run_main(fixture, fixture.argv(**{"--syft-receipt-sha256": digest(fixture.receipt.read_bytes())}))
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())
        for changed_field, changed_value in (
            ("installer_source_sha256", "0" * 64),
            ("verifier", {"lock_path": "syft-darwin-verifier.lock", "lock_sha256": "0" * 64}),
        ):
            with tempfile.TemporaryDirectory() as temporary:
                fixture = ExpectationFixture(Path(temporary))
                fixture.set_target_receipt("Linux", "x86_64")
                bad = json.loads(fixture.receipt.read_bytes())
                bad[changed_field] = changed_value
                fixture.receipt.write_bytes(builder.canonical(bad))
                code, _, _ = self.run_main(
                    fixture,
                    fixture.argv(**{"--syft-receipt-sha256": digest(fixture.receipt.read_bytes())}),
                    host=("Linux", "x86_64"),
                )
                self.assertEqual(code, 1, changed_field)
                self.assertFalse(fixture.output.exists())

    def test_accepts_linux_only_with_native_target_and_lock_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            target = fixture.set_target_receipt("Linux", "x86_64")
            code, stdout, _ = self.run_main(fixture, fixture.argv(), host=("Linux", "x86_64"))
            self.assertEqual(code, 0, stdout)
            prep = json.loads(fixture.preparation_receipt.read_bytes())
            qualification = prep["syft_qualification"]
            self.assertEqual(qualification["platform"], "linux/amd64")
            self.assertEqual(qualification["verifier_lock_path"], builder.SYFT_LINUX_LOCK)
            self.assertEqual(
                qualification["verifier_lock_sha256"],
                digest((fixture.consumer / builder.SYFT_LINUX_LOCK).read_bytes()),
            )
            expected = json.loads((fixture.output / "expected-inputs.json").read_bytes())
            syft_pin = next(item["sha256"] for item in expected["dependencies"] if item["id"] == "tool:syft")
            self.assertEqual(syft_pin, target["binary_sha256"])
            # The fixture rewrites trusted target constants to a fake digest;
            # it tests pin selection and is not native installer qualification.

    def test_rejects_cross_platform_receipt_unsupported_host_and_unpinned_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            fixture.set_target_receipt("Linux", "x86_64")
            code, _, _ = self.run_main(fixture, fixture.argv(), host=("Darwin", "arm64"))
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            code, _, _ = self.run_main(fixture, fixture.argv(), host=("Linux", "aarch64"))
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            fixture.set_target_receipt("Linux", "x86_64")
            fixture.binary.write_bytes(b"second synthetic binary; never executed")
            receipt = json.loads(fixture.receipt.read_bytes())
            receipt["binary_sha256"] = digest(fixture.binary.read_bytes())
            fixture.receipt.write_bytes(builder.canonical(receipt))
            code, _, _ = self.run_main(
                fixture,
                fixture.argv(**{
                    "--syft-sha256": digest(fixture.binary.read_bytes()),
                    "--syft-receipt-sha256": digest(fixture.receipt.read_bytes()),
                }),
                host=("Linux", "x86_64"),
            )
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())

    def test_rejects_tampered_original_zip_and_pin_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            with fixture.acq.archive_zip.open("r+b") as stream:
                stream.seek(-1, 2)
                stream.write(b"x")
            code, _, _ = self.run_main(fixture, fixture.argv())
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            code, _, _ = self.run_main(fixture, fixture.argv(**{"--artifact-id": fixture.acq.artifact_id + 1}))
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())

    def test_rejects_existing_or_input_overlapping_output_without_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            fixture.output.mkdir()
            marker = fixture.output / "keep.txt"
            marker.write_text("keep", encoding="utf-8")
            code, _, _ = self.run_main(fixture, fixture.argv())
            self.assertEqual(code, 1)
            self.assertEqual(marker.read_text(encoding="utf-8"), "keep")
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            fixture.preparation_receipt.write_text("keep", encoding="utf-8")
            code, _, _ = self.run_main(fixture, fixture.argv())
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())
            self.assertEqual(fixture.preparation_receipt.read_text(encoding="utf-8"), "keep")
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            code, _, _ = self.run_main(fixture, fixture.argv(**{"--output-dir": fixture.bundle / "derived"}))
            self.assertEqual(code, 1)
            self.assertFalse((fixture.bundle / "derived").exists())

    def test_late_write_failure_removes_only_fresh_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            preserved = (fixture.acquisition / "receipt.json").read_bytes()
            original = builder.write_exclusive

            def fail_receipt(path: Path, data: bytes) -> None:
                if path == fixture.preparation_receipt:
                    raise OSError("synthetic preparation receipt write failure")
                original(path, data)

            with mock.patch.object(builder, "write_exclusive", side_effect=fail_receipt):
                code, _, _ = self.run_main(fixture, fixture.argv())
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())
            self.assertFalse(fixture.preparation_receipt.exists())
            self.assertEqual((fixture.acquisition / "receipt.json").read_bytes(), preserved)

    def test_partial_preparation_receipt_is_removed_after_fsync_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            original_fsync = builder.os.fsync
            calls = 0

            def fail_fourth_sync(descriptor: int) -> None:
                nonlocal calls
                calls += 1
                if calls == 4:
                    raise OSError("synthetic fsync failure after receipt creation")
                original_fsync(descriptor)

            with mock.patch.object(builder.os, "fsync", side_effect=fail_fourth_sync):
                code, _, _ = self.run_main(fixture, fixture.argv())
            self.assertEqual(calls, 4)
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())
            self.assertFalse(fixture.preparation_receipt.exists())

    def test_rejects_unsupported_native_target_and_result_input(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = ExpectationFixture(Path(temporary))
            code, _, _ = self.run_main(fixture, fixture.argv(), host=("Linux", "aarch64"))
            self.assertEqual(code, 1)
            self.assertFalse(fixture.output.exists())
            code, _, _ = self.run_main(fixture, [*fixture.argv(), "--evidence-dir", str(Path(temporary) / "result")])
            self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main()

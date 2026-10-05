from __future__ import annotations

import hashlib
import io
import json
import re
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts/supply_chain"))
import verify_syft_installation_receipt as verifier


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class ReceiptValidatorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.repo = self.root / "repo"
        self.output = self.root / "output"
        (self.repo / "scripts/supply_chain").mkdir(parents=True)
        (self.output / "evidence").mkdir(parents=True)
        (self.output / "logs").mkdir()
        (self.output / "downloads").mkdir()
        (self.output / "bin").mkdir()
        self.source = b"trusted test installer source"
        self.lock = b"trusted test lock"
        self.retained_linux_target = verifier.TARGETS["linux-amd64"]
        (self.repo / "scripts/supply_chain/install_verified_syft.py").write_bytes(self.source)
        (self.repo / "scripts/supply_chain/syft-linux-verifier.lock").write_bytes(self.lock)
        (self.output / "verifier.lock").write_bytes(self.lock)
        self.binary = b"fixture binary bytes"
        member_payloads = {"CHANGELOG.md": b"fixture changelog", "LICENSE": b"fixture license",
                           "README.md": b"fixture readme", "syft": self.binary}
        archive_buffer = io.BytesIO()
        with tarfile.open(fileobj=archive_buffer, mode="w:gz") as archive:
            for name, payload in member_payloads.items():
                info = tarfile.TarInfo(name)
                info.size = len(payload)
                archive.addfile(info, io.BytesIO(payload))
        self.archive = archive_buffer.getvalue()
        (self.output / "downloads/syft_1.54.0_linux_amd64.tar.gz").write_bytes(self.archive)
        (self.output / "downloads/syft-checksums.txt").write_bytes(b"checksums")
        (self.output / "downloads/syft-checksums.sigstore.json").write_bytes(b"bundle")
        (self.output / "bin/syft").write_bytes(self.binary)
        target = ("linux/amd64", "syft_1.54.0_linux_amd64.tar.gz", sha(self.archive), sha(self.binary),
                  "syft-linux-verifier.lock", sha(self.lock))
        self.pins = patch.multiple(verifier, TARGETS={"linux-amd64": target}, CHECKSUM_SHA256=sha(b"checksums"), BUNDLE_SHA256=sha(b"bundle"))
        self.pins.start()
        self.addCleanup(self.pins.stop)
        members = {"CHANGELOG.md": sha(member_payloads["CHANGELOG.md"]), "LICENSE": sha(member_payloads["LICENSE"]),
                   "README.md": sha(member_payloads["README.md"]), "syft": sha(self.binary)}
        self.archive_receipt = {"member_count": 4, "members": members, "archive_sha256": sha(self.archive),
                                "binary_sha256": sha(self.binary), "expanded_bytes": 42,
                                "metadata_header_limit": 65536, "decompressed_stream_limit": 135352320}
        self.records = []
        self.probe = {"application": "syft", "version": verifier.VERSION, "gitCommit": verifier.COMMIT,
                      "platform": "linux/amd64", "gitDescription": verifier.TAG}
        out = str(self.output)
        for idx, label in enumerate(verifier.LABELS):
            script = str(self.repo / "scripts/supply_chain/install_verified_syft.py")
            if label == "download-checksums":
                argv = ["/usr/bin/python3", script, "--fetch-internal", "https://github.com/anchore/syft/releases/download/v1.54.0/syft_1.54.0_checksums.txt", f"{out}/downloads/syft-checksums.txt", "65536"]
            elif label == "download-signature-bundle":
                argv = ["/usr/bin/python3", script, "--fetch-internal", "https://github.com/anchore/syft/releases/download/v1.54.0/syft_1.54.0_checksums.txt.sigstore.json", f"{out}/downloads/syft-checksums.sigstore.json", "2097152"]
            elif label == "create-verifier-venv":
                argv = ["/usr/bin/python3", "-m", "venv", f"{out}/verifier"]
            elif label == "audit-pip-configuration":
                argv = [f"{out}/verifier/bin/python", f"{out}/pip-config-audit.py", f"{out}/pip-canary.ini"]
            elif label == "install-hash-locked-verifier":
                argv = [f"{out}/verifier/bin/python", "-m", "pip", "--isolated", "--disable-pip-version-check", "--no-input", "install", "--require-hashes", "-r", f"{out}/verifier.lock"]
            elif label == "verify-signed-checksum-document":
                argv = [f"{out}/verifier/bin/python", "-m", "sigstore", "verify", "github", "--bundle", f"{out}/downloads/syft-checksums.sigstore.json", "--cert-identity", verifier.IDENTITY, "--sha", verifier.COMMIT, "--repository", verifier.REPOSITORY, "--ref", verifier.REF, f"{out}/downloads/syft-checksums.txt"]
            elif label == "download-syft-archive":
                argv = ["/usr/bin/python3", script, "--fetch-internal", "https://github.com/anchore/syft/releases/download/v1.54.0/syft_1.54.0_linux_amd64.tar.gz", f"{out}/downloads/syft_1.54.0_linux_amd64.tar.gz", str(verifier.MAX_ARCHIVE)]
            elif label == "extract-syft-archive":
                argv = ["/usr/bin/python3", script, "--extract-internal", f"{out}/downloads/syft_1.54.0_linux_amd64.tar.gz", f"{out}/extract", sha(self.archive)]
            else:
                argv = [f"{out}/bin/syft", "version", "-o", "json"]
            if idx == 7:
                value = {"binary": "syft", "archive": self.archive_receipt}
            elif idx == 8:
                value = self.probe
            else:
                value = None
            if idx == 5:
                log = f"OK: {out}/downloads/syft-checksums.txt\n".encode()
            else:
                log = (json.dumps(value, sort_keys=True).encode() + b"\n") if value is not None else b""
            (self.output / "logs" / f"{idx:02d}-{label}.log").write_bytes(log)
            h = sha(log)
            stdout_hash = sha(b"") if idx in (0, 1, 2, 3, 4, 5, 6) else h
            stderr_hash = h if idx == 5 else sha(b"")
            self.records.append({"label": label, "argv": argv, "exit_status": 0, "log_sha256": h, "log_bytes": len(log),
                                 "stdout_sha256": stdout_hash, "stderr_sha256": stderr_hash})
        self.receipt = {"schema": "kairos.verified-syft-installer.v1", "result": "pass", "version": verifier.VERSION,
                        "release_tag": verifier.TAG, "release_commit": verifier.COMMIT, "repository": verifier.REPOSITORY,
                        "ref": verifier.REF, "certificate_identity": verifier.IDENTITY, "issuer_policy": verifier.ISSUER,
                        "target": "linux-amd64", "platform": "linux/amd64", "authenticated_asset": "syft_1.54.0_linux_amd64.tar.gz",
                        "release_checksum_sha256": verifier.CHECKSUM_SHA256, "release_bundle_sha256": verifier.BUNDLE_SHA256,
                        "signed_asset_sha256": sha(self.archive), "archive_sha256": sha(self.archive), "binary_path": "bin/syft",
                        "binary_sha256": sha(self.binary), "archive": self.archive_receipt,
                        "version_probe": self.probe,
                        "python_toolchain": {"version": "3.14.8 (fixture)"},
                        "verifier": {"sigstore": "4.5.0", "lock_path": "syft-linux-verifier.lock", "lock_sha256": sha(self.lock)},
                        "installer_source_sha256": sha(self.source), "commands": self.records}
        self.write_receipt()

    def tearDown(self):
        self.temp.cleanup()

    def write_receipt(self):
        (self.output / "evidence/receipt.json").write_text(json.dumps(self.receipt), encoding="utf-8")

    def test_accepts_valid_receipt_and_all_nine_logs(self):
        report = verifier.validate(self.output, "linux-amd64", self.repo)
        self.assertEqual(report["result"], "pass")
        self.assertEqual(report["validated_commands"], 9)

    def test_linux_binary_pin_matches_native_qualification_readback(self):
        self.assertEqual(self.retained_linux_target[3], "d46a9a61a6ae3d367f0a03748c5e9c59253e586c4388ab26ddcacebc2efa0d92")

    def test_rejects_binary_digest_that_differs_from_target_pin(self):
        wrong = "0" * 64
        self.receipt["binary_sha256"] = wrong
        self.receipt["archive"]["binary_sha256"] = wrong
        self.receipt["archive"]["members"]["syft"] = wrong
        extraction_log = json.dumps({"binary": "syft", "archive": self.receipt["archive"]}, sort_keys=True).encode() + b"\n"
        (self.output / "logs/07-extract-syft-archive.log").write_bytes(extraction_log)
        command = self.receipt["commands"][7]
        command["log_bytes"] = len(extraction_log)
        command["log_sha256"] = sha(extraction_log)
        command["stdout_sha256"] = sha(extraction_log)
        self.write_receipt()
        with self.assertRaisesRegex(ValueError, "pinned target"):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_workflow_requires_manual_main_and_serial_qualification(self):
        workflow = (ROOT / ".github/workflows/syft-linux-qualification.yml").read_text()
        self.assertIn("concurrency:\n  group: ${{ github.workflow }}-${{ github.ref }}\n  cancel-in-progress: false\n", workflow)
        self.assertIn("on:\n  workflow_dispatch:\n", workflow)
        self.assertNotIn("  push:", workflow)
        self.assertNotIn("  pull_request:", workflow)
        self.assertIn('[[ "$GITHUB_EVENT_NAME" == workflow_dispatch ]]', workflow)
        self.assertIn('[[ "$GITHUB_REPOSITORY" == edithatogo/kairos ]]', workflow)
        self.assertIn('[[ "$GITHUB_REF" == refs/heads/main ]]', workflow)
        self.assertIn("ref: ${{ github.workflow_sha }}", workflow)
        self.assertIn("persist-credentials: false", workflow)
        permissions = re.findall(r"(?m)^([ ]*)permissions:\n((?:[ ]+[A-Za-z-]+: [^\n]+\n)+)", workflow)
        self.assertEqual([(indent, body.strip()) for indent, body in permissions],
                         [("", "contents: read"), ("    ", "contents: read")])

    def test_rejects_changed_release_identity(self):
        self.receipt["certificate_identity"] = "untrusted"
        self.write_receipt()
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_tampered_log(self):
        (self.output / "logs/03-audit-pip-configuration.log").write_bytes(b"changed")
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_missing_log(self):
        (self.output / "logs/02-create-verifier-venv.log").unlink()
        with self.assertRaises(OSError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_symlink_log(self):
        victim = self.root / "victim"
        victim.write_bytes(b"")
        path = self.output / "logs/00-download-checksums.log"
        path.unlink()
        path.symlink_to(victim)
        with self.assertRaises(OSError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_command_failure_or_wrong_order(self):
        self.receipt["commands"][2]["exit_status"] = 1
        self.write_receipt()
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)
        self.receipt["commands"][2]["exit_status"] = 0
        self.receipt["commands"][0], self.receipt["commands"][1] = self.receipt["commands"][1], self.receipt["commands"][0]
        self.write_receipt()
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_path_escape_in_download_command(self):
        self.receipt["commands"][0]["argv"][4] = str(self.output / "../outside")
        self.write_receipt()
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_short_command_before_argument_indexing(self):
        self.receipt["commands"][0]["argv"] = ["python"]
        self.write_receipt()
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_boolean_exit_status_and_uppercase_digest(self):
        self.receipt["commands"][0]["exit_status"] = False
        self.write_receipt()
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)
        self.receipt["commands"][0]["exit_status"] = 0
        self.receipt["commands"][0]["log_sha256"] = "A" * 64
        self.write_receipt()
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_incomplete_archive_member_set(self):
        self.receipt["archive"]["members"].pop("LICENSE")
        self.write_receipt()
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_receipt_symlink(self):
        path = self.output / "evidence/receipt.json"
        content = path.read_bytes()
        path.unlink()
        other = self.root / "receipt-copy"
        other.write_bytes(content)
        path.symlink_to(other)
        with self.assertRaises(OSError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_strict_json_rejects_duplicate_keys_and_non_finite_values(self):
        for raw in (b'{"x": 1, "x": 2}', b'{"x": NaN}', b'{"x": Infinity}', b'{"x": 1e999}'):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                verifier.strict_json(raw)
        with self.assertRaises(ValueError):
            verifier.strict_json(b'[' * 2000 + b'0' + b']' * 2000)

    def validate_bad_archive(self, symlink=False):
        payloads = [("CHANGELOG.md", b"fixture changelog"), ("LICENSE", b"fixture license"),
                    ("README.md", b"fixture readme"), ("syft", self.binary)]
        if not symlink:
            payloads.append(("extra", b"bad"))
        bad_buffer = io.BytesIO()
        with tarfile.open(fileobj=bad_buffer, mode="w:gz") as archive:
            for name, body in payloads:
                info = tarfile.TarInfo(name)
                if symlink and name == "syft":
                    info.type = tarfile.SYMTYPE
                    info.linkname = "../../outside"
                    archive.addfile(info)
                else:
                    info.size = len(body)
                    archive.addfile(info, io.BytesIO(body))
        data = bad_buffer.getvalue()
        old = verifier.TARGETS["linux-amd64"]
        verifier.TARGETS["linux-amd64"] = (old[0], old[1], sha(data), old[3], old[4], old[5])
        self.receipt["signed_asset_sha256"] = sha(data)
        self.receipt["archive_sha256"] = sha(data)
        self.receipt["archive"]["archive_sha256"] = sha(data)
        self.receipt["commands"][7]["argv"][-1] = sha(data)
        extraction_log = json.dumps({"binary": "syft", "archive": self.receipt["archive"]}, sort_keys=True).encode() + b"\n"
        extraction_path = self.output / "logs/07-extract-syft-archive.log"
        extraction_path.write_bytes(extraction_log)
        self.receipt["commands"][7]["log_bytes"] = len(extraction_log)
        self.receipt["commands"][7]["log_sha256"] = sha(extraction_log)
        self.receipt["commands"][7]["stdout_sha256"] = sha(extraction_log)
        (self.output / "downloads/syft_1.54.0_linux_amd64.tar.gz").write_bytes(data)
        self.write_receipt()
        with self.assertRaises(ValueError):
            verifier.validate(self.output, "linux-amd64", self.repo)

    def test_rejects_extra_archive_member(self):
        self.validate_bad_archive()

    def test_rejects_symlink_archive_member(self):
        self.validate_bad_archive(symlink=True)


if __name__ == "__main__":
    unittest.main()

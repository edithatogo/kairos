import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "packaging/scripts"))
import verify_native_beta_evidence as evidence


class NativeBetaEvidenceTest(unittest.TestCase):
    def setUp(self):
        staging = tempfile.TemporaryDirectory()
        self.addCleanup(staging.cleanup)
        self.directory = Path(staging.name)
        self.profile = json.loads(evidence.PROFILE.read_text(encoding="utf-8"))
        self.target = self.profile["platforms"][0]
        self.repository = "owner/example"
        self.candidate = {
            "installed_candidate_run_id": 12,
            "installed_candidate_commit": "a" * 40,
            "installed_candidate_receipt_sha256": {},
        }
        self.run_record = {
            "id": 12,
            "head_sha": "a" * 40,
            "head_branch": "main",
            "path": ".github/workflows/native-beta.yml",
            "event": "push",
            "status": "completed",
            "conclusion": "success",
            "repository": {"full_name": self.repository},
            "head_repository": {"full_name": self.repository},
        }
        names = [evidence.ABI, evidence.SDK, "kairo_ecs-0.1.0-py3-none-any.whl"]
        self.digests = {}
        for name in sorted(names):
            path = self.directory / name
            path.write_text(name, encoding="ascii")
            self.digests[name] = evidence.sha256(path)
        self.installed = {
            "status": "pass",
            "source_commit": "a" * 40,
            "profile": self.profile["profile"],
            "target": self.target,
            "python": self.profile["python"],
            "c_abi": self.profile["c_abi"],
            "rustc": [f"release: {self.profile['rust']}", f"host: {self.target}"],
            "careops_installed": {
                field: self.profile["careops"][field] for field in ("revision", "case")
            },
            "artifacts_sha256": self.digests,
        }
        self.pin_receipt()
        (self.directory / "SHA256SUMS").write_bytes(
            "".join(f"{digest}  {name}\n" for name, digest in self.digests.items()).encode("ascii")
        )

    def pin_receipt(self):
        path = self.directory / evidence.INSTALLATION
        evidence.receipt(path, self.installed)
        self.candidate["installed_candidate_receipt_sha256"][self.target] = evidence.sha256(path)

    def verify(self):
        return evidence.verify_artifacts(self.directory, self.target, self.profile, self.candidate)

    def test_exact_run_and_distributions_pass(self):
        evidence.verify_run(self.run_record, self.candidate, self.repository)
        self.assertEqual(self.verify(), self.digests)

    def test_another_source_trigger_or_repository_is_rejected(self):
        for key, value in (
            ("head_sha", "b" * 40),
            ("event", "pull_request"),
            ("head_repository", {"full_name": "other/example"}),
            ("conclusion", "failure"),
        ):
            with self.subTest(key=key):
                wrong = copy.deepcopy(self.run_record)
                wrong[key] = value
                with self.assertRaisesRegex(SystemExit, key):
                    evidence.verify_run(wrong, self.candidate, self.repository)

    def test_distribution_and_receipt_tampering_are_rejected(self):
        path = self.directory / evidence.ABI
        path.write_bytes(b"tampered")
        with self.assertRaisesRegex(SystemExit, "distribution digest"):
            self.verify()
        path.write_text(evidence.ABI, encoding="ascii")
        self.installed["target"] = self.profile["platforms"][1]
        evidence.receipt(self.directory / evidence.INSTALLATION, self.installed)
        with self.assertRaisesRegex(SystemExit, "receipt digest"):
            self.verify()
        self.pin_receipt()
        with self.assertRaisesRegex(SystemExit, "differs at target"):
            self.verify()

    def test_checksum_substitution_and_unexpected_file_are_rejected(self):
        (self.directory / "SHA256SUMS").write_bytes(b"")
        with self.assertRaisesRegex(SystemExit, "SHA256SUMS"):
            self.verify()
        (self.directory / "unknown").touch()
        with self.assertRaisesRegex(SystemExit, "Unexpected bundle files"):
            self.verify()


if __name__ == "__main__":
    unittest.main()

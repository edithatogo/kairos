from __future__ import annotations

import os
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/archive-supply-chain-main.yml"


def section_after(text: str, marker: str, end_marker: str | None = None) -> str:
    start = text.index(marker)
    search_from = start + len(marker)
    end = text.index(end_marker, search_from) if end_marker else len(text)
    return text[start:end]


def step_python(text: str, step_name: str) -> str:
    step = section_after(text, f"      - name: {step_name}\n", "      - name: ")
    match = re.search(r"python3 -B -(?: \"\$(?:output|root)\")? <<'PY'\n(.*?)\n          PY\n", step, re.DOTALL)
    if match is None:
        raise AssertionError(f"no static Python heredoc in step {step_name!r}")
    return textwrap.dedent(match.group(1))


class ArchiveSupplyChainMainWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")
        cls.acquire = section_after(cls.text, "  acquire:\n", "  scan:\n")
        cls.scan = section_after(cls.text, "  scan:\n")

    def test_manual_only_trusted_workflow_and_cancel_safe_concurrency(self) -> None:
        self.assertIn("on:\n  workflow_dispatch:\n", self.text)
        self.assertNotRegex(self.text, r"(?m)^\s{2}(push|pull_request|schedule|workflow_run):")
        self.assertIn("cancel-in-progress: false", self.text)
        for job in (self.acquire, self.scan):
            self.assertIn('[[ "$GITHUB_EVENT_NAME" == workflow_dispatch ]]', job)
            self.assertIn('[[ "$GITHUB_REPOSITORY" == edithatogo/kairos ]]', job)
            self.assertIn('[[ "$GITHUB_REF" == refs/heads/main ]]', job)
            self.assertIn("ref: ${{ github.workflow_sha }}", job)
            self.assertIn("persist-credentials: false", job)
        self.assertIn("permissions: {}", self.text)
        self.assertIn("    permissions:\n      actions: read", self.acquire)
        self.assertIn("    permissions:\n      contents: read", self.scan)
        self.assertNotIn("GH_TOKEN", self.scan)
        self.assertNotIn("GITHUB_TOKEN", self.scan)
        self.assertNotIn("github.token", self.scan)
        self.assertNotIn("syft", self.acquire.lower())
        self.assertNotIn("build_archive_supply_chain.py", self.acquire)
        self.assertNotRegex(self.text, r"(?m)^\s+(id-token|packages|contents|actions):\s*write\s*$")
        acquire_permissions = re.search(r"(?m)^    permissions:\n((?:      [^\n]+\n)+)", self.acquire)
        scan_permissions = re.search(r"(?m)^    permissions:\n((?:      [^\n]+\n)+)", self.scan)
        self.assertIsNotNone(acquire_permissions)
        self.assertIsNotNone(scan_permissions)
        self.assertEqual(re.sub(r" #[^\n]*", "", acquire_permissions.group(1)), "      actions: read\n      contents: read\n")
        self.assertEqual(re.sub(r" #[^\n]*", "", scan_permissions.group(1)), "      contents: read\n")
        actions = re.findall(r"(?m)^\s+- uses: ([^@\s]+)@([0-9a-f]{40}) # ([^\n]+)$", self.text)
        self.assertEqual(len(actions), 7)
        self.assertTrue(all(version.startswith("v") for _, _, version in actions))

    def test_permissions_explain_each_required_read_scope(self) -> None:
        permissions = re.findall(r"(?m)^      (actions|contents): read(?: # ([^\n]+))?$", self.text)
        self.assertEqual([scope for scope, _ in permissions], ["actions", "contents", "contents"])
        self.assertTrue(all(reason.strip() for _, reason in permissions))

    def test_dispatch_exposes_only_six_required_typed_producer_pins(self) -> None:
        inputs = section_after(self.text, "    inputs:\n", "\npermissions:")
        declarations = re.findall(r"(?m)^      ([a-z0-9_]+):\n(.*?)(?=^      [a-z0-9_]+:\n|\Z)", inputs, re.DOTALL)
        observed = {}
        for name, body in declarations:
            required = re.search(r"(?m)^        required: (true|false)$", body)
            kind = re.search(r"(?m)^        type: ([a-z]+)$", body)
            observed[name] = (required.group(1) if required else None, kind.group(1) if kind else None)
        self.assertEqual(
            observed,
            {
                "run_id": ("true", "number"),
                "artifact_id": ("true", "number"),
                "source_commit": ("true", "string"),
                "producer_tree": ("true", "string"),
                "archive_zip_sha256": ("true", "string"),
                "archive_zip_bytes": ("true", "number"),
            },
        )
        self.assertIn("Validate all six dispatch pins before API access", self.acquire)
        self.assertLess(self.acquire.index("Validate all six dispatch pins before API access"), self.acquire.index("Retain exact run acquisition"))
        self.assertLess(self.acquire.index("Validate all six dispatch pins before API access"), self.acquire.index("actions/checkout@"))
        self.assertIn("--require-main-dispatch --acquisition-output", self.acquire)
        self.assertIn("Reconcile retained acquisition with all six pins", self.acquire)
        self.assertIn("archive_zip_sha256", self.acquire)
        self.assertIn("archive_zip_bytes", self.acquire)
        self.assertIn("workflow_run", self.acquire)
        self.assertIn("artifact-metadata.json", self.acquire)

    def test_dispatch_pin_validator_executes_and_rejects_unsafe_or_partial_values(self) -> None:
        source = step_python(self.text, "Validate all six dispatch pins before API access")
        good = {
            "PIN_RUN_ID": "37200123456",
            "PIN_ARTIFACT_ID": "74500987654",
            "PIN_SOURCE_COMMIT": "a" * 40,
            "PIN_PRODUCER_TREE": "b" * 40,
            "PIN_ARCHIVE_ZIP_SHA256": "c" * 64,
            "PIN_ARCHIVE_ZIP_BYTES": "65000000",
        }
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "github-output"
            base_env = {**os.environ, **good, "GITHUB_OUTPUT": str(output)}
            passed = subprocess.run([sys.executable, "-B", "-"], input=source, text=True,
                                    capture_output=True, env=base_env, check=False)
            self.assertEqual(passed.returncode, 0, passed.stderr)
            lines = output.read_text(encoding="utf-8").splitlines()
            self.assertEqual(lines, [f"{key.removeprefix('PIN_').lower()}={value}" for key, value in good.items()])
            bad_values = (
                {"PIN_RUN_ID": "0"},
                {"PIN_ARTIFACT_ID": "12.0"},
                {"PIN_RUN_ID": "1;print(1)"},
                {"PIN_SOURCE_COMMIT": "A" * 40},
                {"PIN_SOURCE_COMMIT": "a" * 39},
                {"PIN_PRODUCER_TREE": "../" + "a" * 37},
                {"PIN_ARCHIVE_ZIP_SHA256": "C" * 64},
                {"PIN_ARCHIVE_ZIP_SHA256": "c" * 63},
                {"PIN_ARCHIVE_ZIP_BYTES": "0"},
                {"PIN_ARCHIVE_ZIP_BYTES": "67108865"},
                {"PIN_ARTIFACT_ID": "9" * 19},
            )
            for change in bad_values:
                with self.subTest(change=change):
                    output.unlink(missing_ok=True)
                    env = {**base_env, **change}
                    rejected = subprocess.run([sys.executable, "-B", "-"], input=source, text=True,
                                              capture_output=True, env=env, check=False)
                    self.assertNotEqual(rejected.returncode, 0)
                    self.assertFalse(output.exists())
            missing_env = dict(base_env)
            del missing_env["PIN_PRODUCER_TREE"]
            missing = subprocess.run([sys.executable, "-B", "-"], input=source, text=True,
                                     capture_output=True, env=missing_env, check=False)
            self.assertNotEqual(missing.returncode, 0)
            self.assertFalse(output.exists())

    def _acquisition_fixture(self, root: Path) -> dict[str, str]:
        pins = {
            "PIN_RUN_ID": "37200123456",
            "PIN_ARTIFACT_ID": "74500987654",
            "PIN_SOURCE_COMMIT": "a" * 40,
            "PIN_PRODUCER_TREE": "b" * 40,
            "PIN_ARCHIVE_ZIP_SHA256": "",
            "PIN_ARCHIVE_ZIP_BYTES": "0",
        }
        root.mkdir()
        (root / "bundle").mkdir()
        archive_bytes = b"retained exact artifact ZIP bytes"
        pins["PIN_ARCHIVE_ZIP_SHA256"] = hashlib.sha256(archive_bytes).hexdigest()
        pins["PIN_ARCHIVE_ZIP_BYTES"] = str(len(archive_bytes))
        (root / f"{pins['PIN_ARTIFACT_ID']}.zip").write_bytes(archive_bytes)
        (root / "receipt.json").write_text(
            json.dumps({
                "workflow_run": int(pins["PIN_RUN_ID"]),
                "artifact_id": int(pins["PIN_ARTIFACT_ID"]),
                "producer_checkout_source": pins["PIN_SOURCE_COMMIT"],
                "producer_tree": pins["PIN_PRODUCER_TREE"],
                "archive_zip_sha256": pins["PIN_ARCHIVE_ZIP_SHA256"],
                "selection_policy": "same-repository-main-workflow-dispatch",
            })
        )
        (root / "artifact-metadata.json").write_text(
            json.dumps({"id": int(pins["PIN_ARTIFACT_ID"]), "digest": "sha256:" + pins["PIN_ARCHIVE_ZIP_SHA256"]})
        )
        (root / "run-metadata.json").write_text(
            json.dumps({
                "id": int(pins["PIN_RUN_ID"]), "event": "workflow_dispatch", "head_branch": "main",
                "head_sha": pins["PIN_SOURCE_COMMIT"],
                "repository": {"full_name": "edithatogo/kairos"},
                "head_repository": {"full_name": "edithatogo/kairos"},
            })
        )
        (root / "source-commit-readback.json").write_text(
            json.dumps({pins["PIN_SOURCE_COMMIT"]: {"tree": {"sha": pins["PIN_PRODUCER_TREE"]}}})
        )
        (root / "bundle" / "ARCHIVE-INDEX.json").write_text(
            json.dumps({"source_commit": pins["PIN_SOURCE_COMMIT"]})
        )
        return pins

    def _run_pin_reconciler(self, source: str, root: Path, pins: dict[str, str]) -> subprocess.CompletedProcess[str]:
        env = {**os.environ, **pins}
        return subprocess.run([sys.executable, "-B", "-", str(root)], input=source, text=True,
                              capture_output=True, env=env, check=False)

    def test_acquisition_pin_reconciliation_rejects_missing_mismatched_and_duplicate_records(self) -> None:
        source = step_python(self.text, "Reconcile retained acquisition with all six pins")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "acquisition"
            pins = self._acquisition_fixture(root)
            accepted = self._run_pin_reconciler(source, root, pins)
            self.assertEqual(accepted.returncode, 0, accepted.stderr)
            original = (root / "receipt.json").read_text()
            for changed in (
                original.replace(pins["PIN_PRODUCER_TREE"], "d" * 40),
                original.replace(pins["PIN_ARCHIVE_ZIP_SHA256"], "e" * 64),
                original.replace(pins["PIN_SOURCE_COMMIT"], "f" * 40),
                original.replace('"artifact_id": 74500987654', '"artifact_id": true'),
                original.replace("{", '{"workflow_run": 1, ', 1),
            ):
                with self.subTest(receipt=changed[:100]):
                    (root / "receipt.json").write_text(changed)
                    rejected = self._run_pin_reconciler(source, root, pins)
                    self.assertNotEqual(rejected.returncode, 0)
            (root / "receipt.json").write_text(original)
            (root / f"{pins['PIN_ARTIFACT_ID']}.zip").unlink()
            missing = self._run_pin_reconciler(source, root, pins)
            self.assertNotEqual(missing.returncode, 0)

    def test_job_handoff_pin_reconciliation_executes_and_rejects_tampering(self) -> None:
        source = step_python(self.text, "Recheck producer pins after job handoff")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "acquisition"
            pins = self._acquisition_fixture(root)
            accepted = self._run_pin_reconciler(source, root, pins)
            self.assertEqual(accepted.returncode, 0, accepted.stderr)
            zip_path = root / f"{pins['PIN_ARTIFACT_ID']}.zip"
            original = zip_path.read_bytes()
            zip_path.write_bytes(original + b"tamper")
            rejected = self._run_pin_reconciler(source, root, pins)
            self.assertNotEqual(rejected.returncode, 0)
            zip_path.write_bytes(original)
            (root / "artifact-metadata.json").write_text(
                json.dumps({"id": int(pins["PIN_ARTIFACT_ID"]), "digest": "sha256:" + "0" * 64})
            )
            rejected = self._run_pin_reconciler(source, root, pins)
            self.assertNotEqual(rejected.returncode, 0)

    def test_job_order_keeps_expectations_before_scanning_and_verification_before_upload(self) -> None:
        self.assertLess(self.scan.index("actions/download-artifact@"), self.scan.index("Recheck producer pins after job handoff"))
        self.assertLess(self.scan.index("Recheck producer pins after job handoff"), self.scan.index("Authenticate and install pinned Syft"))
        self.assertLess(self.scan.index("Independently validate native installation evidence"), self.scan.index("Prepare independent archive and tool expectations before scanning"))
        self.assertLess(self.scan.index("Prepare independent archive and tool expectations before scanning"), self.scan.index("Run pinned offline archive scanner and evidence builder"))
        self.assertLess(self.scan.index("Run pinned offline archive scanner and evidence builder"), self.scan.index("Verify the complete generated archive evidence profile"))
        self.assertLess(self.scan.index("Verify the complete generated archive evidence profile"), self.scan.index("actions/upload-artifact@"))
        self.assertIn("--target linux-amd64", self.scan)
        self.assertIn("python-version: 3.14.8", self.scan)
        self.assertIn('[[ "$(uname -m)" == x86_64 ]]', self.scan)
        self.assertIn("d46a9a61a6ae3d367f0a03748c5e9c59253e586c4388ab26ddcacebc2efa0d92", self.scan)
        self.assertIn("--expected-binding", self.scan)
        self.assertIn("--expected-inputs", self.scan)
        self.assertIn('--target linux-amd64 --output-dir "$output"', self.scan)

    def test_retained_artifacts_include_full_replayable_evidence_without_installed_tool_bytes(self) -> None:
        final_upload = self.scan[self.scan.rindex("      - uses: actions/upload-artifact@"):]
        self.assertIn("/archive-evidence-${{ github.run_id }}-${{ github.run_attempt }}/evidence/", final_upload)
        self.assertIn("/archive-evidence-${{ github.run_id }}-${{ github.run_attempt }}/expectations/", final_upload)
        self.assertIn("/archive-evidence-${{ github.run_id }}-${{ github.run_attempt }}/preparation-receipt.json", final_upload)
        self.assertIn("/archive-evidence-${{ github.run_id }}-${{ github.run_attempt }}/validation-report.json", final_upload)
        self.assertIn("/archive-acquisition-${{ github.run_id }}-${{ github.run_attempt }}/receipt.json", final_upload)
        self.assertIn("/syft-linux-amd64-${{ github.run_id }}-${{ github.run_attempt }}/evidence/receipt.json", final_upload)
        self.assertIn("/syft-linux-amd64-${{ github.run_id }}-${{ github.run_attempt }}/logs/*.log", final_upload)
        for excluded in ("/bin/syft", "/downloads/", ".zip", "/verifier/", "/archives/"):
            self.assertNotIn(excluded, final_upload)
        self.assertEqual(self.text.count("retention-days: 30"), 2)
        self.assertNotIn("retention-days: 7", self.text)

    def test_generator_uses_prepared_acquisition_adapter_and_its_digest(self) -> None:
        generator = section_after(
            self.scan,
            "      - name: Run pinned offline archive scanner and evidence builder\n",
            "      - name: Verify the complete generated archive evidence profile\n",
        )
        self.assertIn('acquisition="$work/expectations/acquisition.json"', generator)
        self.assertIn('acquisition_sha256="$(python3 -B -c', generator)
        self.assertIn('"$acquisition")"', generator)
        self.assertIn('--input "$acquisition_dir/bundle"', generator)
        self.assertIn('--acquisition "$acquisition"', generator)
        self.assertNotIn('--acquisition "$acquisition_dir/acquisition.json"', generator)


if __name__ == "__main__":
    unittest.main()

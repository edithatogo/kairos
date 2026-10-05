"""Static fail-closed contract for retained archive release preparation."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]


class MainlineArchiveReleaseWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = (ROOT / ".github/workflows/release.yml").read_text()
        cls.actual = cls.workflow.split("  actual-archive-preparation:\n", 1)[1].split("\n  release-dry-run:\n", 1)[0]
        cls.legacy = cls.workflow.split("\n  release-dry-run:\n", 1)[1]

    def test_required_explicit_pin_documents(self):
        for key in ("profile", "dry_run"):
            block = re.search(rf"(?m)^      {key}:\n((?:        [^\n]*\n)+)", self.workflow)
            self.assertIsNotNone(block, key)
            self.assertIn("        required: true\n", block.group(1))
        for key in ("producer_pins", "consumer_pins"):
            self.assertIn(f"      {key}:\n", self.workflow)
        self.assertIn("if not raw or len(raw) > 4096", self.actual)

    def test_main_only_dry_run_exact_checkout(self):
        for value in ('[[ "$GITHUB_EVENT_NAME" == workflow_dispatch ]]', '[[ "$GITHUB_REPOSITORY" == edithatogo/kairos ]]', '[[ "$GITHUB_REF" == refs/heads/main ]]', '[[ "$DRY_RUN" == true ]]', 'ref: ${{ github.workflow_sha }}', 'persist-credentials: false', 'cancel-in-progress: false'):
            self.assertIn(value, self.workflow)

    def test_native_pinned_tools_and_locked_dependencies(self):
        self.assertIn("runs-on: ubuntu-24.04", self.workflow)
        self.assertIn("python-version: '3.14.8'", self.workflow)
        self.assertIn("--require-hashes -r scripts/archive-python-tools.lock", self.workflow)
        self.assertLess(self.workflow.index("Install hash-locked"), self.workflow.index("packaging/scripts/prepare_mainline_archive_release.py"))

    def test_input_expressions_are_env_data(self):
        self.assertIn("PRODUCER_PINS_JSON: ${{ inputs.producer_pins }}", self.workflow)
        self.assertIn("CONSUMER_PINS_JSON: ${{ inputs.consumer_pins }}", self.workflow)
        self.assertIn("len(raw) > 4096", self.workflow)
        for line in self.workflow.splitlines():
            if "${{ inputs." in line:
                self.assertRegex(line, r"^          [A-Z_]+:")

    def test_orchestrator_receives_all_explicit_inputs(self):
        for option in ("--repository", "--release-source-commit", "--producer-pins-json", "--consumer-pins-json", "--work-dir", "--output"):
            self.assertIn(option, self.workflow)
        self.assertIn('--release-source-commit "$GITHUB_SHA"', self.workflow)

    def test_no_rebuild_rescan_or_publication_commands(self):
        for value in ("cargo ", "rustup ", "publish", "build_release_manifest.py", "build_archive_supply_chain.py", "setup-node@", "setup-dotnet@"):
            self.assertNotIn(value, self.actual)

    def test_gate_precedes_retention(self):
        self.assertLess(self.workflow.index("packaging/scripts/prepare_mainline_archive_release.py"), self.workflow.index("actions/upload-artifact@"))
        for value in ("if: always()", "continue-on-error"):
            self.assertNotIn(value, self.workflow)
        for value in ("if-no-files-found: error", "retention-days: 90", "/archive-release-preparation-"):
            self.assertIn(value, self.workflow)

    def test_retention_excludes_fresh_binary_downloads_and_venv(self):
        upload = self.actual.split("      - name: Upload qualified", 1)[1]
        for value in ("/actual-package-archives/", "/producer-acquisition/", "/consumer-acquisition/", "/expectations/", "/fresh-syft/evidence/receipt.json", "/command-records.json", "/expectation-preparation.json", "/actual-archive-readback-report.json"):
            self.assertIn(value, upload)
        for value in ("/fresh-syft/bin/", "/fresh-syft/downloads/", "/fresh-syft/verifier-venv/", "/fresh-syft/\n"):
            self.assertNotIn(value, upload)

    def test_legacy_gates_remain_default_and_separate(self):
        self.assertIn("default: legacy-source-inventory", self.workflow)
        self.assertIn("if: inputs.profile == 'legacy-source-inventory'", self.legacy)
        self.assertIn("if: inputs.profile == 'actual-package-archives'", self.actual)
        for path in ("tests/test_track15_release_evidence_gate.ps1", "scripts/validate_track15_release_delivery.ps1"):
            self.assertIn(path, self.legacy)
            self.assertNotIn(path, self.actual)
        self.assertIn("build_release_manifest.py --verify-existing", self.legacy)

    def test_unknown_profile_cannot_skip_every_gate(self):
        guard = self.workflow.split("  validate-profile:\n", 1)[1].split("  actual-archive-preparation:\n", 1)[0]
        self.assertIn('[[ "$PROFILE" == legacy-source-inventory || "$PROFILE" == actual-package-archives ]]', guard)
        self.assertIn("needs: validate-profile", self.actual)
        self.assertIn("needs: validate-profile", self.legacy)


if __name__ == "__main__":
    unittest.main()

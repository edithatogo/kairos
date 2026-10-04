"""Fail-closed contract tests for the focused archive regression workflow."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW_PATH = ".github/workflows/archive-supply-chain-regression.yml"
EXPECTED_PATHS = {
    WORKFLOW_PATH,
    ".github/workflows/ci-policy.yml",
    ".github/workflows/workflow-security.yml",
    "scripts/validation/validate-track13-metadata.mjs",
    "scripts/archive-supply-chain-test-tools.in",
    "scripts/archive-supply-chain-test-tools.lock",
    "packaging/scripts/acquire_package_archive_bundle.py",
    "packaging/scripts/build_archive_supply_chain.py",
    "packaging/scripts/build_archive_release_manifest.py",
    "packaging/scripts/build_package_archive_bundle.py",
    "tests/test_archive_supply_chain.py",
    "tests/test_archive_supply_chain_ci.py",
    "tests/test_package_archive_acquisition.py",
    "packaging/scripts/install_syft_linux.py",
    "tests/test_install_syft_linux.py",
    "tests/fixtures/archive-supply-chain/**",
}
EXPECTED_ACTIONS = [
    "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
    "actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97",
]
EXPECTED_TEST_COMMANDS = [
    "python -m unittest discover -s tests -p 'test_archive_supply_chain*.py' -v",
    "python -m unittest discover -s tests -p 'test_package_archive_acquisition.py' -v",
    "python -m unittest discover -s tests -p 'test_install_syft_linux.py' -v",
]
EXPECTED_RUN_COMMANDS = [
    "python -m pip install --require-hashes -r scripts/archive-supply-chain-test-tools.lock",
    *EXPECTED_TEST_COMMANDS,
]


def top_level_block(document, key):
    lines = document.splitlines()
    start = next((index for index, line in enumerate(lines) if line == f"{key}:"), None)
    if start is None:
        return []
    result = [lines[start]]
    for line in lines[start + 1 :]:
        if line and not line[0].isspace() and not line.lstrip().startswith("#"):
            break
        result.append(line)
    return result


def path_blocks(on_block):
    matches = re.findall(r"(?m)^    paths:\n((?:^      - .*\n?)+)", "\n".join(on_block))
    return [
        {line.strip()[2:].strip("'\"") for line in block.splitlines() if line.strip().startswith("-")}
        for block in matches
    ]


def contract_errors(workflow, ci_policy, workflow_security, track13_validator):
    errors = []
    on_block = top_level_block(workflow, "on")
    events = re.findall(r"(?m)^  ([a-z][a-z0-9_-]*):(?:\s|$)", "\n".join(on_block))
    if set(events) != {"pull_request", "push", "workflow_dispatch"}:
        errors.append("events")
    if "    branches: [main]" not in "\n".join(on_block):
        errors.append("push-branch")
    paths = path_blocks(on_block)
    if len(paths) != 2 or any(group != EXPECTED_PATHS for group in paths):
        errors.append("path-filters")

    permissions = top_level_block(workflow, "permissions")
    permission_values = [line.strip() for line in permissions[1:] if line.strip()]
    if permission_values != ["contents: read"]:
        errors.append("permissions")

    actions = re.findall(r"(?m)^      - uses: ([^\s]+)", workflow)
    if actions != EXPECTED_ACTIONS:
        errors.append("action-pins")
    if "persist-credentials: false" not in workflow:
        errors.append("checkout-credentials")
    if "runs-on: ubuntu-24.04" not in workflow:
        errors.append("runner-image")
    if "python-version: '3.14.8'" not in workflow:
        errors.append("python-version")
    if "PIP_CONFIG_FILE: /dev/null" not in workflow or "PIP_DISABLE_PIP_VERSION_CHECK: '1'" not in workflow:
        errors.append("pip-environment")

    run_commands = re.findall(r"(?m)^        run:\s*(.*?)\s*$", workflow)
    if run_commands != EXPECTED_RUN_COMMANDS:
        errors.append("test-selectors")
    forbidden = (
        "workflow_run:",
        "pull_request_target:",
        "actions/download-artifact",
        "actions/attest",
        "gh api",
        "GITHUB_TOKEN",
        "secrets.",
        "id-token:",
        "attestations:",
        "actions: write",
        "packages: write",
        "contents: write",
        "just ci",
        "cargo test --workspace",
        "syft ",
    )
    if any(token in workflow for token in forbidden):
        errors.append("forbidden-effect")

    required_inventory = f"test -f {WORKFLOW_PATH}"
    if required_inventory not in ci_policy:
        errors.append("ci-policy-inventory")
    if required_inventory not in workflow_security:
        errors.append("workflow-security-inventory")
    if f"'{Path(WORKFLOW_PATH).name}'" not in track13_validator:
        errors.append("track13-workflow-list")
    if "workflowFiles" not in track13_validator or "ciPolicyText.includes(workflowPath" not in track13_validator or "workflowSecurityText.includes(workflowPath" not in track13_validator:
        errors.append("track13-dynamic-inventory")
    return errors


class ArchiveSupplyChainWorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = (ROOT / WORKFLOW_PATH).read_text()
        cls.ci_policy = (ROOT / ".github/workflows/ci-policy.yml").read_text()
        cls.workflow_security = (ROOT / ".github/workflows/workflow-security.yml").read_text()
        cls.track13_validator = (ROOT / "scripts/validation/validate-track13-metadata.mjs").read_text()

    def test_workflow_matches_focused_read_only_contract(self):
        self.assertEqual(
            contract_errors(self.workflow, self.ci_policy, self.workflow_security, self.track13_validator),
            [],
        )

    def test_rejects_privilege_trigger_pin_and_test_scope_mutations(self):
        mutations = [
            (self.workflow.replace("contents: read", "contents: write", 1), self.ci_policy, self.workflow_security, self.track13_validator),
            (self.workflow.replace("  workflow_dispatch:\n", "  workflow_dispatch:\n  workflow_run:\n", 1), self.ci_policy, self.workflow_security, self.track13_validator),
            (self.workflow.replace(EXPECTED_ACTIONS[0], "actions/checkout@main", 1), self.ci_policy, self.workflow_security, self.track13_validator),
            (self.workflow.replace("      - 'packaging/scripts/acquire_package_archive_bundle.py'\n", "", 1), self.ci_policy, self.workflow_security, self.track13_validator),
            (self.workflow.replace("      - 'packaging/scripts/install_syft_linux.py'\n", "", 1), self.ci_policy, self.workflow_security, self.track13_validator),
            (self.workflow.replace("      - 'tests/test_install_syft_linux.py'\n", "", 1), self.ci_policy, self.workflow_security, self.track13_validator),
            (
                self.workflow.replace(
                    "      - name: Test bounded package archive acquisition\n",
                    "      - name: Accidental full test suite\n        run: python -m unittest discover -s tests -v\n"
                    "      - name: Test bounded package archive acquisition\n",
                    1,
                ),
                self.ci_policy,
                self.workflow_security,
                self.track13_validator,
            ),
        ]
        for index, args in enumerate(mutations):
            with self.subTest(mutation=index):
                self.assertTrue(contract_errors(*args))

    def test_rejects_missing_required_inventory_entries(self):
        mutations = [
            (self.workflow, self.ci_policy.replace(f"          {f'test -f {WORKFLOW_PATH}'}\n", "", 1), self.workflow_security, self.track13_validator),
            (self.workflow, self.ci_policy, self.workflow_security.replace(f"          {f'test -f {WORKFLOW_PATH}'}\n", "", 1), self.track13_validator),
            (self.workflow, self.ci_policy, self.workflow_security, self.track13_validator.replace(f"  '{Path(WORKFLOW_PATH).name}',\n", "", 1)),
        ]
        for index, args in enumerate(mutations):
            with self.subTest(inventory=index):
                self.assertTrue(contract_errors(*args))


if __name__ == "__main__":
    unittest.main()

"""Behavioral tests for the narrowly scoped EXC-193 npm audit classifier."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import sys
import unittest
from datetime import datetime
from pathlib import Path
from zoneinfo import ZoneInfo


ROOT = Path(__file__).resolve().parents[1]
EXCEPTIONS = (
    ROOT
    / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions"
)
POLICY_PATH = EXCEPTIONS / "npm_audit_policy.py"


def _load_policy_module():
    # Import from the owned source path so this suite remains independent of
    # package layout and does not silently skip when the implementation is absent.
    spec = importlib.util.spec_from_file_location("npm_audit_policy", POLICY_PATH)
    if spec is None or spec.loader is None:
        raise ModuleNotFoundError(f"EXC-193 policy module is missing: {POLICY_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class NpmAuditExceptionPolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.classifier = _load_policy_module().classify
        cls.baseline = json.loads(
            (EXCEPTIONS / "EXC-193-http-cache-audit-baseline.json").read_text()
        )
        cls.proposed_policy = json.loads(
            (EXCEPTIONS / "EXC-193-http-cache.json").read_text()
        )
        cls.now = datetime(2026, 10, 4, 12, 0, tzinfo=ZoneInfo("Australia/Brisbane"))

    def approved_policy(self):
        policy = copy.deepcopy(self.proposed_policy)
        policy["status"] = "approved"
        policy["classification"] = "temporary_operational_exception"
        approvals = policy["approvals"]
        approvals.update(
            {
                "security_owner": "security-owner",
                "release_owner": "release-owner",
                "classification_accepted": True,
                "approved_at": "2026-10-03T00:00:00+10:00",
                "approval_evidence": "userapproval",
            }
        )
        return policy

    def classify(self, report=None, raw_exit=1, policy=None, context="development_pr_193", pull_request=193):
        return self.classifier(
            copy.deepcopy(self.baseline if report is None else report),
            raw_exit,
            copy.deepcopy(self.approved_policy() if policy is None else policy),
            context,
            pull_request,
            now=self.now,
        )

    def assert_rejected(self, **kwargs):
        with self.assertRaises(ValueError):
            self.classify(**kwargs)

    def _refresh_graph_fingerprint(self, report, policy):
        encoded = json.dumps(
            report["vulnerabilities"], sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
        policy["vulnerabilities_sha256"] = hashlib.sha256(encoded).hexdigest()

    def test_exact_approved_finding_graph_is_a_temporary_exception(self):
        for context in (
            "development_pr_193",
            "alpha_package_dry_run",
            "beta_package_dry_run",
        ):
            with self.subTest(context=context):
                self.assertEqual(
                    self.classify(context=context), "approved_temporary_exception"
                )

    def test_zero_finding_successful_audit_is_clean_without_approval(self):
        report = copy.deepcopy(self.baseline)
        report["vulnerabilities"] = {}
        report["metadata"]["vulnerabilities"] = {
            "info": 0,
            "low": 0,
            "moderate": 0,
            "high": 0,
            "critical": 0,
            "total": 0,
        }
        pending_policy = copy.deepcopy(self.proposed_policy)

        self.assertEqual(
            self.classify(report=report, raw_exit=0, policy=pending_policy), "clean"
        )

    def test_pending_expired_and_not_yet_effective_approvals_reject(self):
        pending = self.approved_policy()
        pending["status"] = "proposed"
        pending["classification"] = "temporary_operational_exception_pending_owner_classification"
        pending["approvals"].update(
            {
                "security_owner": None,
                "release_owner": None,
                "classification_accepted": False,
                "approved_at": None,
                "approval_evidence": None,
            }
        )
        self.assert_rejected(policy=pending)

        expired = self.approved_policy()
        self.now = datetime(2026, 10, 10, 0, 0, tzinfo=ZoneInfo("Australia/Brisbane"))
        self.assert_rejected(policy=expired)
        self.now = datetime(2026, 10, 4, 12, 0, tzinfo=ZoneInfo("Australia/Brisbane"))

        future = self.approved_policy()
        future["approvals"]["approved_at"] = "2026-10-05T00:00:00+10:00"
        self.assert_rejected(policy=future)

    def test_approvals_require_named_owners_classification_and_evidence(self):
        for field, value in (
            ("security_owner", ""),
            ("security_owner", "   "),
            ("release_owner", None),
            ("release_owner", 193),
            ("classification_accepted", False),
            ("classification_accepted", "true"),
            ("approval_evidence", None),
            ("approval_evidence", "  "),
        ):
            with self.subTest(field=field, value=value):
                policy = self.approved_policy()
                policy["approvals"][field] = value
                self.assert_rejected(policy=policy)

    def test_scope_requires_both_the_bound_pr_and_an_allowed_context(self):
        self.assert_rejected(pull_request=194)
        self.assert_rejected(context="development_pr_194")
        self.assert_rejected(pull_request=True)
        for context in ("publication", "release_candidate", "1.0"):
            with self.subTest(context=context):
                self.assert_rejected(context=context)

    def test_non_one_audit_exit_and_boolean_exit_cannot_use_exception(self):
        self.assert_rejected(raw_exit=0)
        self.assert_rejected(raw_exit=2)
        # bool is an int subclass in Python, but True must not stand in for npm's exit 1.
        self.assert_rejected(raw_exit=True)

    def test_scan_errors_malformed_json_and_unsupported_schema_fail_closed(self):
        for report, raw_exit in (
            ({"error": {"code": "ENETWORK", "summary": "registry unavailable"}}, 1),
            ({"error": {"code": "EAUTH", "summary": "authentication failed"}}, 0),
            ({"auditReportVersion": 2, "vulnerabilities": []}, 1),
            ({"auditReportVersion": True, "vulnerabilities": {}, "metadata": {}}, 1),
            ({"auditReportVersion": 3, "vulnerabilities": {}, "metadata": {}}, 0),
        ):
            with self.subTest(report=report, raw_exit=raw_exit):
                self.assert_rejected(report=report, raw_exit=raw_exit)

    def test_metadata_counts_must_agree_with_the_finding_graph(self):
        empty_graph_with_nonzero_counts = copy.deepcopy(self.baseline)
        empty_graph_with_nonzero_counts["vulnerabilities"] = {}
        self.assert_rejected(report=empty_graph_with_nonzero_counts)

        nonempty_graph_with_zero_counts = copy.deepcopy(self.baseline)
        nonempty_graph_with_zero_counts["metadata"]["vulnerabilities"] = {
            key: 0
            for key in ("info", "low", "moderate", "high", "critical", "total")
        }
        self.assert_rejected(report=nonempty_graph_with_zero_counts)

        for metadata in (None, [], {"vulnerabilities": None}, {"vulnerabilities": {"total": True}}):
            with self.subTest(metadata=metadata):
                report = copy.deepcopy(self.baseline)
                report["metadata"] = metadata
                self.assert_rejected(report=report)

    def test_any_additional_advisory_at_any_severity_rejects(self):
        for severity in ("info", "low", "moderate", "high", "critical"):
            with self.subTest(severity=severity):
                report = copy.deepcopy(self.baseline)
                report["vulnerabilities"]["unrelated-package"] = {
                    "name": "unrelated-package",
                    "severity": severity,
                    "isDirect": True,
                    "via": [
                        {
                            "source": 999001,
                            "name": "unrelated-package",
                            "severity": severity,
                            "range": "<1.0.0",
                            "url": "https://github.com/advisories/GHSA-aaaa-bbbb-cccc",
                        }
                    ],
                    "effects": [],
                    "range": "<1.0.0",
                    "nodes": ["node_modules/unrelated-package"],
                    "fixAvailable": False,
                }
                summary = report["metadata"]["vulnerabilities"]
                summary[severity] += 1
                summary["total"] += 1
                self.assert_rejected(report=report)

    def test_leaf_package_cannot_carry_a_second_advisory(self):
        report = copy.deepcopy(self.baseline)
        report["vulnerabilities"]["http-cache-semantics"]["via"].append(
            {
                "source": 999002,
                "name": "http-cache-semantics",
                "severity": "moderate",
                "range": "<4.3.0",
                "url": "https://github.com/advisories/GHSA-dddd-eeee-ffff",
            }
        )
        self.assert_rejected(report=report)

    def test_leaf_graph_node_severity_and_range_are_part_of_the_allowlist(self):
        mutations = (
            lambda leaf: leaf["nodes"].append("node_modules/another-copy/http-cache-semantics"),
            lambda leaf: leaf.__setitem__("severity", "critical"),
            lambda leaf: leaf["via"][0].__setitem__("range", "<4.3.0"),
        )
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                report = copy.deepcopy(self.baseline)
                mutate(report["vulnerabilities"]["http-cache-semantics"])
                self.assert_rejected(report=report)

    def test_leaf_advisory_identity_must_match_even_with_matching_fingerprint(self):
        report = copy.deepcopy(self.baseline)
        policy = self.approved_policy()
        via = report["vulnerabilities"]["http-cache-semantics"]["via"][0]
        via["source"] = 999003
        via["url"] = "https://github.com/advisories/GHSA-aaaa-bbbb-cccc"
        self._refresh_graph_fingerprint(report, policy)

        self.assert_rejected(report=report, policy=policy)

    def test_policy_cannot_expand_its_allowed_contexts(self):
        policy = self.approved_policy()
        policy["allowed_contexts"].append("publication")
        self.assert_rejected(policy=policy, context="publication")

        # In Python, True compares equal to 1; the policy PR identifier is an
        # integer contract and must not be replaced by a boolean value.
        policy = self.approved_policy()
        policy["required_pull_request"] = True
        self.assert_rejected(policy=policy, pull_request=True)


if __name__ == "__main__":
    unittest.main()

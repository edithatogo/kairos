"""Fail-closed classifier for the exact EXC-193 and EXC-199 npm audit records.

The immutable policy fingerprints bind each approved record. Scope is selected
from trusted repository, PR number and head-ref values; event labels alone do
not authorize an exception. This module never reads files or changes raw audit
results.
"""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from typing import Any

_GRAPH_FINGERPRINT = "0b3e5f1d5f65b48f1a20618ba352e6f02529a134f62e0230126ac68c73b5fec8"
_ADVISORY = {
    "source": 1240991,
    "name": "http-cache-semantics",
    "dependency": "http-cache-semantics",
    "url": "https://github.com/advisories/GHSA-ch52-4w7c-c8xp",
    "severity": "high",
    "range": "<=4.2.0",
}
_SEVERITIES = ("info", "low", "moderate", "high", "critical")
_COUNT_KEYS = frozenset((*_SEVERITIES, "total"))

# These fingerprints pin each approved record independently. EXC-193 retains
# its original source binding; EXC-199 carries the separately approved corrected
# source amendment and cannot reuse the EXC-193 proof set.
_POLICY_RECORDS: dict[str, dict[str, Any]] = {
    "EXC-193": {
        "policy_path": "EXC-193-http-cache.json",
        "record_sha256": "9c0264efc73e4287104b09b48ff6f11543552bc91ca96981a0cffe2921724c71",
        "raw_record_sha256": "52b441437ef8a962cb761d25fede81ce04a0c07ead1aced02ab05606969fe917",
        "required_pull_request": 193,
        "repository": "edithatogo/kairos",
        "head_ref": "codex/kairos-implementation-programme",
        "allowed_contexts": frozenset({"development_pr_193", "alpha_package_dry_run", "beta_package_dry_run"}),
        "excluded_contexts": frozenset({"release_candidate", "1.0", "publication", "other_dependency_trees", "website_dependency_tree"}),
        "patched_index_sha256": "fc7b3f0265b7a7d0fee83bafa47186a66495720d3179801c2be3083de6d0cf76",
        "file_sha256": {
            "scripts/bootstrap-node-tools/package-lock.json": "3905b6f36ea3b5625667f3a40a829e8eb7a351f6ff074372863f02b1d2216552",
            "scripts/bootstrap-node-tools/apply_http_cache_fix.py": "a928e08eacca08e497199ab58747ff751041643748a0f53ff988dbd7d2d0aa91",
            "scripts/bootstrap-node-tools/validate_npm_cli.mjs": "3757fcb8d2bc16ba842cc5f3868c1f09de591ef50a599f56c83ab1d51397b9a4",
            "tests/test_http_cache_patch.py": "542a970f0cf79242334b274cd93ec60ee32fb05b375d2302591cb822550479b9",
            "tests/http-cache-security-regression.mjs": "298c3537d14c19afdc188cde554596d4f2fc4f95c56d387b79af9ab851f510b2",
        },
        "mitigation_review_status": None,
    },
    "EXC-199": {
        "policy_path": "EXC-199-http-cache.json",
        "record_sha256": "352f4dfc079355772ac1c92509158385e646f817b30148a2201bfd656c22d231",
        "raw_record_sha256": "aa7f45487764f103e78126b7279eee132cb40d0cdbd0445d606902a34e9d8ba4",
        "required_pull_request": 199,
        "repository": "edithatogo/kairos",
        "head_ref": "codex/kairos-track48-optimistic-runtime",
        "allowed_contexts": frozenset({"development_pr_199", "alpha_package_dry_run", "beta_package_dry_run"}),
        "excluded_contexts": frozenset({"release_candidate", "1.0", "publication", "other_dependency_trees", "website_dependency_tree", "other_pull_requests"}),
        "patched_index_sha256": "5942c6d3df40fce2151d8e409e7ad7e7c9c4a8ee09b7066072edf3a939fc589c",
        "file_sha256": {
            "scripts/bootstrap-node-tools/package-lock.json": "3905b6f36ea3b5625667f3a40a829e8eb7a351f6ff074372863f02b1d2216552",
            "scripts/bootstrap-node-tools/apply_http_cache_fix.py": "1745f11f6b2ae27c47ba00218970192ec0b0034d3d1467b524e411e2c3c9afa4",
            "scripts/bootstrap-node-tools/validate_npm_cli.mjs": "68361630ff540c9e32e1e62417c805b54c35b57195f5108c7679b6ca4c15bcb8",
            "tests/test_http_cache_patch.py": "4701a42699573255b6dac6c0815585137ac7e6132c8f2ebe3e7ddb95b9b0a641",
            "tests/http-cache-security-regression.mjs": "05d4c9990c5dfc691798336443d28638a405a751076ae7147efc7a19a5392d9c",
        },
        "source_binding_amendment": {
            "id": "EXC-199-mitigation-amendment",
            "status": "human_approved",
            "record": "EXC-199-mitigation-amendment.json",
            "record_sha256": "b4818bfc43ad2f2061087735489ffbc488b3d42f30dd599f44a7787df2a3c212",
            "evidence_manifest": "evidence/EXC-199-amendment/manifest.json",
            "evidence_manifest_sha256": "e311be230541cc961c1428ab151f62a0a45baa01edacb97f9bbca04713e932e7",
            "mitigation_source_commit": "2efedc5ea04c1caf26a200bd2450e9e38426e8fe",
            "patched_index_sha256": "5942c6d3df40fce2151d8e409e7ad7e7c9c4a8ee09b7066072edf3a939fc589c",
            "file_sha256": {
                "scripts/bootstrap-node-tools/package-lock.json": "3905b6f36ea3b5625667f3a40a829e8eb7a351f6ff074372863f02b1d2216552",
                "scripts/bootstrap-node-tools/apply_http_cache_fix.py": "1745f11f6b2ae27c47ba00218970192ec0b0034d3d1467b524e411e2c3c9afa4",
                "scripts/bootstrap-node-tools/validate_npm_cli.mjs": "68361630ff540c9e32e1e62417c805b54c35b57195f5108c7679b6ca4c15bcb8",
                "tests/test_http_cache_patch.py": "4701a42699573255b6dac6c0815585137ac7e6132c8f2ebe3e7ddb95b9b0a641",
                "tests/http-cache-security-regression.mjs": "05d4c9990c5dfc691798336443d28638a405a751076ae7147efc7a19a5392d9c",
            },
            "review_status": "reviewed_corrected_source",
            "owner_approval": {
                "security_owner": "human sole maintainer (this chat; repository account edithatogo)",
                "release_owner": "human sole maintainer (this chat; repository account edithatogo)",
                "approved_at": "2026-10-04T10:39:27.598427+10:00",
                "approval_evidence": "Human user replied “I approve” to the explicit source-binding amendment question for corrected mitigation commit2efedc5ea04c1caf26a200bd2450e9e38426e8fe and patched index5942c6d3df40fce2151d8e409e7ad7e7c9c4a8ee09b7066072edf3a939fc589c. Scope and expiry unchanged; no RC/1.0/publication/website/other PR or tree approval. Timestamp records receipt of this decision, not a claimed message-send time."
            }
        },
        "mitigation_review_status": "reviewed_corrected_source",
    },
}


def _reject(message: str) -> None:
    raise ValueError(message)


def exception_for_identity(pull_request: int, repository: str, head_ref: str) -> tuple[str, dict[str, Any]]:
    """Resolve only a statically approved PR/repository/head-ref conjunction."""
    if isinstance(pull_request, bool) or not isinstance(pull_request, int):
        _reject("pull request number must be an integer")
    if not isinstance(repository, str) or not isinstance(head_ref, str):
        _reject("repository and head ref are required")
    for exception_id, record in _POLICY_RECORDS.items():
        if (
            pull_request == record["required_pull_request"]
            and repository == record["repository"]
            and head_ref == record["head_ref"]
        ):
            return exception_id, record
    _reject("no approved exception mapping for this repository, PR and head ref")


def _aware_time(value: Any, field: str) -> datetime:
    if not isinstance(value, str) or not value.strip():
        _reject(f"{field} must be a non-empty ISO datetime")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (TypeError, ValueError):
        _reject(f"{field} must be a valid ISO datetime")
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        _reject(f"{field} must include a timezone")
    return parsed.astimezone(timezone.utc)


def _validate_counts(report: dict[str, Any], vulnerabilities: dict[str, Any]) -> dict[str, int]:
    metadata = report.get("metadata")
    if not isinstance(metadata, dict) or not isinstance(metadata.get("vulnerabilities"), dict):
        _reject("report is missing vulnerability counts")
    counts = metadata["vulnerabilities"]
    if set(counts) != _COUNT_KEYS:
        _reject("vulnerability counts must contain exactly the severity counters and total")
    normalized: dict[str, int] = {}
    for key in _COUNT_KEYS:
        value = counts[key]
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            _reject(f"vulnerability count {key} must be a non-negative integer")
        normalized[key] = value
    actual = {severity: 0 for severity in _SEVERITIES}
    for row in vulnerabilities.values():
        severity = row.get("severity") if isinstance(row, dict) else None
        if isinstance(severity, str) and severity in actual:
            actual[severity] += 1
    actual["total"] = len(vulnerabilities)
    if normalized != actual:
        _reject("vulnerability counts do not match the report rows")
    return normalized


def _validate_graph(vulnerabilities: dict[str, Any]) -> None:
    if len(vulnerabilities) != 19:
        _reject("the approved graph must contain exactly 19 rows")
    for name, row in vulnerabilities.items():
        if (
            not isinstance(name, str)
            or not name
            or not isinstance(row, dict)
            or row.get("name") != name
            or row.get("severity") != "high"
        ):
            _reject("each graph row must have its matching name and high severity")
        nodes = row.get("nodes")
        if not isinstance(nodes, list) or not nodes or any(not isinstance(node, str) or not node for node in nodes):
            _reject("each graph row must have a non-empty list of node paths")
        via = row.get("via")
        if not isinstance(via, list) or not via:
            _reject("each graph row must have a non-empty via list")

    visiting: set[str] = set()
    memo: dict[str, bool] = {}

    def reaches_advisory(name: str) -> bool:
        if name in visiting:
            _reject("the vulnerability graph contains a cycle")
        if name in memo:
            return memo[name]
        visiting.add(name)
        has_leaf = False
        for item in vulnerabilities[name]["via"]:
            if isinstance(item, str):
                if item not in vulnerabilities:
                    _reject("the vulnerability graph references a missing row")
                if reaches_advisory(item):
                    has_leaf = True
            elif isinstance(item, dict):
                for key, expected in _ADVISORY.items():
                    if item.get(key) != expected:
                        _reject("the graph contains an unexpected advisory")
                has_leaf = True
            else:
                _reject("via entries must be row references or advisory objects")
        visiting.remove(name)
        memo[name] = has_leaf
        return has_leaf

    for name in vulnerabilities:
        if not reaches_advisory(name):
            _reject("every graph row must trace to the approved advisory")
    leaves = [item for row in vulnerabilities.values() for item in row["via"] if isinstance(item, dict)]
    if len(leaves) != 1:
        _reject("the graph must have exactly one advisory leaf")


def _validate_approval(
    policy: dict[str, Any],
    exception_id: str,
    record: dict[str, Any],
    context: str,
    pull_request: int,
    repository: str,
    head_ref: str,
    now: datetime,
) -> None:
    if policy.get("status") != "approved":
        _reject("the exception is not approved")
    if policy.get("classification") != "temporary_operational_exception":
        _reject("the policy classification is not approved")
    approvals = policy.get("approvals")
    if not isinstance(approvals, dict):
        _reject("approval record is missing")
    for owner in ("security_owner", "release_owner"):
        value = approvals.get(owner)
        if not isinstance(value, str) or not value.strip():
            _reject(f"{owner} must be named")
    if approvals.get("classification_accepted") is not True:
        _reject("the classification has not been accepted")
    evidence = approvals.get("approval_evidence")
    if not isinstance(evidence, str) or not evidence.strip():
        _reject("approval evidence is required")
    approved_at = _aware_time(approvals.get("approved_at"), "approved_at")
    expires_at = _aware_time(policy.get("expires_at"), "expires_at")
    current = now.astimezone(timezone.utc)
    if approved_at > current:
        _reject("approval is dated in the future")
    if current >= expires_at:
        _reject("the exception has expired")
    if approved_at >= expires_at:
        _reject("approval must precede expiry")
    if exception_id == "EXC-199":
        amendment = policy.get("source_binding_amendment")
        source_approval = amendment.get("owner_approval") if isinstance(amendment, dict) else None
        if not isinstance(source_approval, dict):
            _reject("source-binding amendment approval is missing")
        for owner in ("security_owner", "release_owner"):
            value = source_approval.get(owner)
            if not isinstance(value, str) or not value.strip():
                _reject(f"source-binding {owner} must be named")
        source_evidence = source_approval.get("approval_evidence")
        if not isinstance(source_evidence, str) or not source_evidence.strip():
            _reject("source-binding approval evidence is required")
        source_approved_at = _aware_time(source_approval.get("approved_at"), "source_binding_approved_at")
        if source_approved_at > current:
            _reject("source-binding approval is dated in the future")
        if source_approved_at >= expires_at:
            _reject("source-binding approval must precede expiry")

    if isinstance(pull_request, bool) or not isinstance(pull_request, int):
        _reject("actual pull request number must be an integer")
    if pull_request != record["required_pull_request"]:
        _reject(f"the actual pull request must be EXC-{pull_request} policy's required PR")
    policy_pr = policy.get("required_pull_request")
    if isinstance(policy_pr, bool) or not isinstance(policy_pr, int) or policy_pr != record["required_pull_request"]:
        _reject("the policy's required pull request is invalid")
    policy_repository = policy.get("repository")
    if repository != record["repository"] or (policy_repository is not None and policy_repository != record["repository"]):
        _reject("the repository is outside the approved exception scope")
    if head_ref != record["head_ref"] or policy.get("head_ref") != record["head_ref"]:
        _reject("the head ref is outside the approved exception scope")
    allowed = policy.get("allowed_contexts")
    excluded = policy.get("excluded_contexts")
    if not isinstance(allowed, list) or len(allowed) != len(set(allowed)) or set(allowed) != record["allowed_contexts"]:
        _reject("policy contexts differ from the immutable scope")
    if not isinstance(excluded, list) or len(excluded) != len(set(excluded)) or set(excluded) != record["excluded_contexts"]:
        _reject("excluded contexts differ from the immutable scope")
    if not isinstance(context, str) or context not in record["allowed_contexts"]:
        _reject("context is outside the conjunctive exception scope")
    if context in record["excluded_contexts"]:
        _reject("context is explicitly excluded")
    if record["mitigation_review_status"] is not None:
        if policy.get("mitigation_review_status") != record["mitigation_review_status"]:
            _reject("mitigation review status differs from the immutable record")
        if record["mitigation_review_status"] == "blocked_stale_fallback_gap":
            _reject("EXC-199 mitigation review is blocked by the stale-fallback gap")


def _classify_reviewed_graph(
    report: dict[str, Any],
    raw_exit: int,
    policy: dict[str, Any],
    context: str,
    pull_request: int,
    repository: str,
    head_ref: str,
    record: dict[str, Any],
    now: datetime,
) -> str:
    """Classify after record pinning and proof review have succeeded.

    Tests may exercise a hypothetical reviewed record by replacing the private
    record fixture. Production callers only reach this after immutable pin and
    source-evidence validation in ``classify`` and the runner.
    """
    if isinstance(raw_exit, bool) or not isinstance(raw_exit, int) or raw_exit != 1:
        _reject("a non-empty audit requires raw exit 1")
    vulnerabilities = report.get("vulnerabilities")
    if not isinstance(vulnerabilities, dict):
        _reject("vulnerabilities must be an object")
    fingerprint = hashlib.sha256(
        json.dumps(vulnerabilities, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    if fingerprint != _GRAPH_FINGERPRINT or policy.get("vulnerabilities_sha256") != _GRAPH_FINGERPRINT:
        _reject("the vulnerability graph differs from the reviewed baseline")
    _validate_graph(vulnerabilities)
    _validate_approval(policy, policy["id"], record, context, pull_request, repository, head_ref, now)
    return "approved_temporary_exception"


def validate_audit_result(report: dict, raw_exit: int) -> bool:
    """Validate raw npm report structure and counts; return whether it is clean.

    This check is independent of exception records and is safe to run before
    looking at exception-only source proofs. It never turns a finding into a
    pass; callers must classify non-empty reports separately.
    """
    if not isinstance(report, dict):
        _reject("audit report must be an object")
    if "error" in report or report.get("errors"):
        _reject("audit report contains an error")
    version = report.get("auditReportVersion")
    if isinstance(version, bool) or not isinstance(version, int) or version != 2:
        _reject("unsupported audit report version")
    vulnerabilities = report.get("vulnerabilities")
    if not isinstance(vulnerabilities, dict):
        _reject("vulnerabilities must be an object")
    counts = _validate_counts(report, vulnerabilities)

    if not vulnerabilities:
        if isinstance(raw_exit, bool) or not isinstance(raw_exit, int) or raw_exit != 0 or any(counts.values()):
            _reject("a clean audit requires exit 0 and zero vulnerability counts")
        return True
    if isinstance(raw_exit, bool) or not isinstance(raw_exit, int) or raw_exit != 1:
        _reject("a non-empty audit requires raw exit 1")
    _validate_graph(vulnerabilities)
    return False


def classify(
    report: dict,
    raw_exit: int,
    policy: dict,
    context: str,
    pull_request: int,
    now: datetime | None = None,
    repository: str | None = None,
    head_ref: str | None = None,
) -> str:
    """Return ``clean`` or a narrowly scoped exception; preserve raw exit."""
    if not isinstance(policy, dict):
        _reject("policy must be an object")

    # A genuinely clean audit is a strict pass and does not consume an exception.
    if validate_audit_result(report, raw_exit):
        return "clean"
    vulnerabilities = report["vulnerabilities"]

    if repository is None or head_ref is None:
        _reject("repository and head ref are required for exception classification")
    exception_id = policy.get("id")
    record = _POLICY_RECORDS.get(exception_id)
    if record is None:
        _reject("exception record is not in the immutable allowlist")
    canonical_record = json.dumps(policy, sort_keys=True, separators=(",", ":")).encode("utf-8")
    if hashlib.sha256(canonical_record).hexdigest() != record["record_sha256"]:
        _reject("policy record differs from the immutable approved record")
    expected_fields = {
        "head_ref": record["head_ref"],
        "required_pull_request": record["required_pull_request"],
        "raw_audit_command": [
            "node", "scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js", "audit",
            "--prefix", "scripts/bootstrap-node-tools", "--audit-level=moderate", "--json",
        ],
        "vulnerabilities_sha256": _GRAPH_FINGERPRINT,
        "file_sha256": record["file_sha256"],
        "patched_index_sha256": record["patched_index_sha256"],
        "expires_at": "2026-10-10T00:00:00+10:00",
    }
    if exception_id == "EXC-199":
        expected_fields["source_binding_amendment"] = record["source_binding_amendment"]
    for key, expected in expected_fields.items():
        if policy.get(key) != expected:
            _reject(f"immutable policy field drift: {key}")
    if record["mitigation_review_status"] is not None:
        if policy.get("mitigation_review_status") != record["mitigation_review_status"]:
            _reject("mitigation review status differs from the immutable record")

    current = now if now is not None else datetime.now(timezone.utc)
    if not isinstance(current, datetime) or current.tzinfo is None or current.utcoffset() is None:
        _reject("now must be a timezone-aware datetime")
    return _classify_reviewed_graph(
        report, raw_exit, policy, context, pull_request, repository, head_ref, record, current
    )

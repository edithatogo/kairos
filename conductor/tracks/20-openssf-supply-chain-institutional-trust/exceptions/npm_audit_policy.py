"""Fail-closed policy classifier for the EXC-193 npm audit record.

This module is pure: callers supply the parsed report, policy record, context,
and PR number. It never reads files, contacts a registry, or changes raw audit
results.
"""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from typing import Any


_BASELINE_FINGERPRINT = (
    "0b3e5f1d5f65b48f1a20618ba352e6f02529a134f62e0230126ac68c73b5fec8"
)
_REQUIRED_PR = 193
_ALLOWED_CONTEXTS = frozenset(
    {"development_pr_193", "alpha_package_dry_run", "beta_package_dry_run"}
)
_EXCLUDED_CONTEXTS = frozenset(
    {"release_candidate", "1.0", "publication", "other_dependency_trees", "website_dependency_tree"}
)
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


def _reject(message: str) -> None:
    raise ValueError(message)

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
        _reject("the EXC-193 graph must contain exactly 19 rows")
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
            _reject("every graph row must trace to the EXC-193 advisory")

    leaves = [
        item
        for row in vulnerabilities.values()
        for item in row["via"]
        if isinstance(item, dict)
    ]
    if len(leaves) != 1:
        _reject("the graph must contain exactly one advisory leaf")

def _validate_approval(policy: dict[str, Any], context: str, pull_request: int, now: datetime) -> None:
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

    if isinstance(pull_request, bool) or not isinstance(pull_request, int) or pull_request != _REQUIRED_PR:
        _reject("the actual pull request must be EXC-193's required PR")
    required_pr = policy.get("required_pull_request")
    if isinstance(required_pr, bool) or not isinstance(required_pr, int) or required_pr != _REQUIRED_PR:
        _reject("the policy's required pull request is invalid")
    if not isinstance(context, str):
        _reject("context must be a string")
    allowed = policy.get("allowed_contexts")
    excluded = policy.get("excluded_contexts")
    if not isinstance(allowed, list) or not isinstance(excluded, list):
        _reject("policy context scope is malformed")
    if context not in _ALLOWED_CONTEXTS or context not in allowed or context in _EXCLUDED_CONTEXTS or context in excluded:
        _reject("context is outside the conjunctive EXC-193 scope")

def classify(
    report: dict,
    raw_exit: int,
    policy: dict,
    context: str,
    pull_request: int,
    now: datetime | None = None,
) -> str:
    """Return ``clean`` or ``approved_temporary_exception``; reject all else."""
    if not isinstance(report, dict):
        _reject("audit report must be an object")
    if not isinstance(policy, dict):
        _reject("policy must be an object")
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
        return "clean"

    if isinstance(raw_exit, bool) or not isinstance(raw_exit, int) or raw_exit != 1:
        _reject("a non-empty audit requires raw exit 1")
    current = now if now is not None else datetime.now(timezone.utc)
    if not isinstance(current, datetime) or current.tzinfo is None or current.utcoffset() is None:
        _reject("now must be a timezone-aware datetime")

    fingerprint = hashlib.sha256(
        json.dumps(vulnerabilities, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    if fingerprint != _BASELINE_FINGERPRINT or policy.get("vulnerabilities_sha256") != _BASELINE_FINGERPRINT:
        _reject("the vulnerability graph differs from the reviewed baseline")
    _validate_graph(vulnerabilities)
    _validate_approval(policy, context, pull_request, current)
    return "approved_temporary_exception"

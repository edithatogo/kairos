# Track 112 specification: Capability evidence and claim automation

## Overview and approved outcome

Keep public claims traceable to precise evidence and prevent placeholders, historical receipts or interface-shape tests from becoming false acceptance.

User authority: 7 November 2026 request for immediate Rust-only support, accurate current documentation, complete scientific methods, performance improvement, explicit beta/stable gates and Kairos as an independent CareOps dependency.

## Inputs

Reviewed baseline d56863505d938feb069af88a737eb7668113d14b; existing contracts and original acceptance receipts; [detailed programme](../../frontier-programme-20261107.md); [capability register](../../capabilities.json); [dataset catalogue](../../validation-datasets.md).

## Outputs

Reviewed implementation/qualification packets and versioned evidence for the phase outcomes below. Plans do not assert runtime implementation or beta/release acceptance.

## Owned paths and interfaces

This track owns its planning/evidence directory. Source scopes are coordinated with the original owners before implementation: conductor/capabilities.json, conductor/capabilities.md, scripts/validation/, tests/conformance/. Public API/ABI, RNG/schema, native lifetime or result changes require the existing ADR/conformance/versioning process. Immediate host-core retirement is explicitly authorised and has no migration grace period.

## Dependencies and parallel-safe tracks

Depends on existing/new Tracks 12, 13, 14, 18, 21 for the named contracts. Work may proceed on independent subpackets after contract review; dependency closure is required for final acceptance. Disjoint worktrees and repository-wide claims prevent shared-checkout writers. No source owner or consumer pin is silently taken over.

## Blocked paths

Unrelated worktrees, private credentials/data, external consumer implementation/pins and protected branch settings. Release publication and hardware/service provisioning have separate authority.

## Acceptance criteria and quality gates

- Integrity: Changing a local result byte makes its digest check fail.
- False claims: Synthetic/local-placeholder data cannot satisfy a measured speedup claim; source-only evidence cannot qualify native execution.
- Drift: Generated capability table must match its JSON; dangling evidence and track IDs fail.
- Scope: Validator explicitly states it validates evidence structure, not the truth of external experiments or new scientific results.

Required common gates: phase-closeout-check, frontier-evidence-check, original owner-specific native/scientific/security checks and independent review. The test matrix specifies evidence boundaries.

## Risks and release implications

A schema validator cannot prove a reported experiment occurred. Require actual raw execution evidence and independent review; never turn metadata success into performance, clinical or release acceptance.

Only accepted capabilities may appear in supported beta/stable scope. Historical Done records are not transferred to new algorithms, platforms, dependencies or consumers. See [release gates](../../release-gates.md).


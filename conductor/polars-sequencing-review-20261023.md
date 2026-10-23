# Review Report: inventory-first Polars sequencing

## Summary

Planning-only revision follows the user decision to start Polars inventory and gate only demonstrated dependent tasks. No implementation phase is advanced.

## Verification Checks

- [x] Plan compliance: removed blanket Kairos 93/94/96 and CareOps R11 prerequisite edges; preserved ownership, existing contracts, reconciliation R3 and upstream Phase 4 receipts.
- [x] Style compliance: separate dependency tracks and explicit evidence boundaries retained.
- [x] New tests: none; no runtime change. Existing planning validators used.
- [x] Test coverage: registry/DAG and planning consistency only; no runtime or hosted acceptance claimed.
- [x] Test results: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`: exit 0, no errors/warnings; 105 tracks. `git diff --check`: exit 0.

## Scope and evidence

Working directory: `/Volumes/PortableSSD/codex-worktrees/kairos-polars-sequencing`. Base commit: `a929da5dc674a1f74645f40f545f35130e726974`. Review performed in the coordinator session using the Conductor review procedure; not a separate-agent review. Tool versions are recorded below. No simulation seed applies to this documentation revision. Source changes and validation are reproducible from this commit; this file is the planning receipt. Full Rust CI and runtime tests skipped because only Conductor documents/metadata changed.

## Review decisions

Each inventory target must have removal/retention rationale, owner and exact dependent task/receipt/oracle before affected dispatch. No blanket adoption is implied, and no removed target is marked implemented. Existing tests and Polars acceptance requirements remain mandatory. Historical sequencing review is explicitly superseded. Separate claims protect unrelated FMI and other runtime work. Machine readiness remains preparation eligibility, not runtime dispatch authority.

Tool versions: Python 3.15.0; PowerShell 7.6.8.

## Main-branch integration review — 23 October 2026

Publication worktree: kairos-publish-sequencing; original review and its historical tool versions above are preserved. Fresh validation uses the explicitly selected repo-local .venv/bin/python, verified as Python 3.15.1. The diff remains confined to Conductor planning and preserves runtime sources, submodule pins, implementation checkboxes and acceptance status.

Reconciled against main 2ba9fb9712f005a071ef3f81cfda6b122837ed34. Resolved plan/handoff conflicts by retaining the existing Phase 0/1 history and adding prospective task-specific gates; no retroactive certification. Fresh phase-gate and DAG validators pass (105 tracks, zero errors/warnings); git diff --check passes. The quality-frontier readback reports drift on existing main CodeQL and Codecov project evidence, plus pending Renovate refresh. No settings or acceptance waivers. Full just ci is not run locally for this documentation-only slice; the required hosted PR checks remain mandatory.

PR review identified stale precedence wording in dependency-value-policy.md. Corrected in both repositories so the higher-precedence value policy also requires inventory-first, task-specific gates. Acceptance obligations and separate tracks are unchanged.
The broader hosted Conductor no-skip validator fails on missing gate catalogue entries in quality-gates.md. Reproduced read-only on unchanged main 2ba9fb9 (exit 1); the registry gate names, validator and catalogue are unchanged by this PR. The catalogue remains reserved by the active toolchain owner. This is a pre-existing, non-required hosted check failure, not a passing validation claim. Required GitHub checks remain mandatory on the final PR head.

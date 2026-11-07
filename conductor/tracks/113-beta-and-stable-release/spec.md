# Track 113 specification: Beta and stable release qualification

## Overview and approved outcome

Release a precisely scoped beta, then an independently reproducible and compatible stable core, without implying all experimental modules are complete.

User authority: 7 November 2026 request for immediate Rust-only support, accurate current documentation, complete scientific methods, performance improvement, explicit beta/stable gates and Kairos as an independent CareOps dependency.

## Inputs

Reviewed baseline d56863505d938feb069af88a737eb7668113d14b; existing contracts and original acceptance receipts; [detailed programme](../../release-gates.md); [capability register](../../capabilities.json); [dataset catalogue](../../validation-datasets.md).

## Outputs

Reviewed implementation/qualification packets and versioned evidence for the phase outcomes below. Plans do not assert runtime implementation or beta/release acceptance.

## Owned paths and interfaces

This track owns its planning/evidence directory. Source scopes are coordinated with the original owners before implementation: packaging/, docs/release/, conductor/release-gates.md. Public API/ABI, RNG/schema, native lifetime or result changes require the existing ADR/conformance/versioning process. Immediate host-core retirement is explicitly authorised and has no migration grace period.

## Dependencies and parallel-safe tracks

Depends on existing/new Tracks 15, 16, 20, 25, 109, 110, 111, 112, 114 for the named contracts. Work may proceed on independent subpackets after contract review; dependency closure is required for final acceptance. Disjoint worktrees and repository-wide claims prevent shared-checkout writers. No source owner or consumer pin is silently taken over.

## Blocked paths

Unrelated worktrees, private credentials/data, external consumer implementation/pins and protected branch settings. Release publication and hardware/service provisioning have separate authority.

## Acceptance criteria and quality gates

- Beta: Every enumerated required gate has candidate-specific evidence or the capability is excluded.
- Installed packages: Tests execute distributed artefacts in clean advertised environments, not only editable source.
- Stable: Independent reproduction, upgrade/replay fixtures, supported limits and no unresolved critical/high defects in supported scope.
- Publication: Actual tag/registry/provenance readback is distinct from CI or PR success.

Required common gates: phase-closeout-check, frontier-evidence-check, original owner-specific native/scientific/security checks and independent review. The test matrix specifies evidence boundaries.

## Risks and release implications

Stable branding can overgeneralize experimental scope. Version the supported manifest and keep experimental crates/features separately labelled; do not infer registry publication from packaging scripts.

Only accepted capabilities may appear in supported beta/stable scope. Historical Done records are not transferred to new algorithms, platforms, dependencies or consumers. See [release gates](../../release-gates.md).


# Track 110 specification: Scientific method and advanced capability completion

## Overview and approved outcome

Close every stated scientific/advanced method with a correct implementation and independent evidence, preserving explicit unsupported boundaries until qualification.

User authority: 7 November 2026 request for immediate Rust-only support, accurate current documentation, complete scientific methods, performance improvement, explicit beta/stable gates and Kairos as an independent CareOps dependency.

## Inputs

Reviewed baseline d56863505d938feb069af88a737eb7668113d14b; existing contracts and original acceptance receipts; [detailed programme](../../scientific-completion.md); [capability register](../../capabilities.json); [dataset catalogue](../../validation-datasets.md).

## Outputs

Reviewed implementation/qualification packets and versioned evidence for the phase outcomes below. Plans do not assert runtime implementation or beta/release acceptance.

## Owned paths and interfaces

This track owns its planning/evidence directory. Source scopes are coordinated with the original owners before implementation: crates/kairo-ecs-game-theory/, crates/kairo-ecs-ml/, crates/kairo-ecs-rng/. Public API/ABI, RNG/schema, native lifetime or result changes require the existing ADR/conformance/versioning process. Immediate host-core retirement is explicitly authorised and has no migration grace period.

## Dependencies and parallel-safe tracks

Depends on existing/new Tracks 01, 12, 21, 65, 66 for the named contracts. Work may proceed on independent subpackets after contract review; dependency closure is required for final acceptance. Disjoint worktrees and repository-wide claims prevent shared-checkout writers. No source owner or consumer pin is silently taken over.

## Blocked paths

Unrelated worktrees, private credentials/data, external consumer implementation/pins and protected branch settings. Release publication and hardware/service provisioning have separate authority.

## Acceptance criteria and quality gates

- S01–S08: Use the named independent analytical/game/QMC/statistical oracles in scientific-completion.md, including degenerate and boundary cases.
- S09–S10: Nonidentity outputs and real state transitions originate in Rust; scalar/batched equivalence, reset/seed/termination and lifetime tests.
- S11–S17: Actual devices/transports/external FMUs/filesystems or explicit capability-unavailable result; no mock or fixture metadata accepted as native proof.
- Scientific claims: Every supported method has a measured residual/error criterion and held-out test; classify each unsupported method explicitly.

Required common gates: phase-closeout-check, frontier-evidence-check, original owner-specific native/scientific/security checks and independent review. The test matrix specifies evidence boundaries.

## Risks and release implications

Method fixes change saved results. Preserve legacy algorithm identity for interpretation while retiring unsupported claims; use independent oracles instead of reproducing the implementation in tests.

Only accepted capabilities may appear in supported beta/stable scope. Historical Done records are not transferred to new algorithms, platforms, dependencies or consumers. See [release gates](../../release-gates.md).


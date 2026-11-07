# Track 114 specification: Maintained dependencies and independent consumer contracts

## Overview and approved outcome

Reduce custom maintenance through qualified reuse and make Kairos a standalone dependency consumed by CareOps through explicit public contracts.

User authority: 7 November 2026 request for immediate Rust-only support, accurate current documentation, complete scientific methods, performance improvement, explicit beta/stable gates and Kairos as an independent CareOps dependency.

## Inputs

Reviewed baseline d56863505d938feb069af88a737eb7668113d14b; existing contracts and original acceptance receipts; [detailed programme](../../dependency-and-consumer-contracts.md); [capability register](../../capabilities.json); [dataset catalogue](../../validation-datasets.md).

## Outputs

Reviewed implementation/qualification packets and versioned evidence for the phase outcomes below. Plans do not assert runtime implementation or beta/release acceptance.

## Owned paths and interfaces

This track owns its planning/evidence directory. Source scopes are coordinated with the original owners before implementation: Cargo.toml, Cargo.lock, conductor/dependency-and-consumer-contracts.md. Public API/ABI, RNG/schema, native lifetime or result changes require the existing ADR/conformance/versioning process. Immediate host-core retirement is explicitly authorised and has no migration grace period.

## Dependencies and parallel-safe tracks

Depends on existing/new Tracks 01, 02, 04, 25, 70, 108 for the named contracts. Work may proceed on independent subpackets after contract review; dependency closure is required for final acceptance. Disjoint worktrees and repository-wide claims prevent shared-checkout writers. No source owner or consumer pin is silently taken over.

## Blocked paths

Unrelated worktrees, private credentials/data, external consumer implementation/pins and protected branch settings. Release publication and hardware/service provisioning have separate authority.

## Acceptance criteria and quality gates

- Dependency direction: Kairos production manifest/source cannot require CareOps; minimal consumer fixture runs without the CareOps repository.
- Consumer behaviour: Installed CareOps uses native provider capability and explicit immutable version; unsupported provider fails, no reference fallback.
- Feature closure: Minimal core excludes optional data/GPU/MPI/HTTP/binding stacks; optional profiles compile and function.
- Adoption: Old and new implementations match scientific/runtime contracts and measured value; licensing/security accepted.

Required common gates: phase-closeout-check, frontier-evidence-check, original owner-specific native/scientific/security checks and independent review. The test matrix specifies evidence boundaries.

## Risks and release implications

A convenient framework can alter identities, order or build footprint. Preserve public contracts; consumer pin changes require separate actual downstream qualification.

Only accepted capabilities may appear in supported beta/stable scope. Historical Done records are not transferred to new algorithms, platforms, dependencies or consumers. See [release gates](../../release-gates.md).


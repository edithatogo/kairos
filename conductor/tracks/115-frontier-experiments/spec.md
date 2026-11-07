# Track 115 specification: Frontier research experiments and reproducible evidence

## Overview and approved outcome

Evaluate specific research advances beyond established ingredients with falsifiable hypotheses, held-out workloads and publicly reproducible evidence.

User authority: 7 November 2026 request for immediate Rust-only support, accurate current documentation, complete scientific methods, performance improvement, explicit beta/stable gates and Kairos as an independent CareOps dependency.

## Inputs

Reviewed baseline d56863505d938feb069af88a737eb7668113d14b; existing contracts and original acceptance receipts; [detailed programme](../../frontier-programme-20261107.md); [capability register](../../capabilities.json); [dataset catalogue](../../validation-datasets.md).

## Outputs

Reviewed implementation/qualification packets and versioned evidence for the phase outcomes below. Plans do not assert runtime implementation or beta/release acceptance.

## Owned paths and interfaces

This track owns its planning/evidence directory. Source scopes are coordinated with the original owners before implementation: paper/, benches/, conductor/tracks/115-frontier-experiments/. Public API/ABI, RNG/schema, native lifetime or result changes require the existing ADR/conformance/versioning process. Immediate host-core retirement is explicitly authorised and has no migration grace period.

## Dependencies and parallel-safe tracks

Depends on existing/new Tracks 110, 111, 112, 114 for the named contracts. Work may proceed on independent subpackets after contract review; dependency closure is required for final acceptance. Disjoint worktrees and repository-wide claims prevent shared-checkout writers. No source owner or consumer pin is silently taken over.

## Blocked paths

Unrelated worktrees, private credentials/data, external consumer implementation/pins and protected branch settings. Release publication and hardware/service provisioning have separate authority.

## Acceptance criteria and quality gates

- Novelty: Specific claim survives current primary-source comparator review; generic Rust/ECS/GPU/TimeWarp labels are insufficient.
- Held-out effects: Independent cases support predeclared endpoints and uncertainty; ablation identifies mechanism.
- Reproducibility: A second runner reproduces accepted bounded results from exact public permitted artefacts.
- Claims: No superiority/causal/clinical-validity statement exceeds evaluated workload and evidence.

Required common gates: phase-closeout-check, frontier-evidence-check, original owner-specific native/scientific/security checks and independent review. The test matrix specifies evidence boundaries.

## Risks and release implications

Feature breadth can outrun scientific support. Cap scope to falsifiable experiments, retain negative results and do not promise exceeding every competitor.

Only accepted capabilities may appear in supported beta/stable scope. Historical Done records are not transferred to new algorithms, platforms, dependencies or consumers. See [release gates](../../release-gates.md).


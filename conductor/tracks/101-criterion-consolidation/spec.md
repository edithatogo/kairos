# criterion: existing-tooling

Planning only; adoption pending. [Value policy](../../dependency-value-policy.md), [sequencing](../../dependency-sequencing.md).

## Inputs

Accepted local dependencies none; source/owner/value qualification still required. Exact live consumer inventory and independent baseline.

## Outputs

Accepted improve/adopt/defer/reject decision and bounded implementation receipts if justified. Existing-tooling optimisation is not a new library installation success criterion.

## Owned paths

Lead benchmark-agent: Existing benchmark targets and their owned Cargo dev-dependencies; current benchmark and PDES/game-theory owners must hand off. Planning owns this folder; runtime/manifest/lock/CI changes require explicit current-owner handoff and claims.

## Blocked paths

Unrelated owners, cross-repository source, clinical policy, DES/resource semantics and protected settings.

## Dependencies

[]. Conditional branches do not gate the core migration. No network/ODE work solely to satisfy a library roadmap.

## Contract

Consolidate applicable ad hoc microbenchmark timing in game-theory/PDES onto Criterion, already used by kairo-ecs-bench/debug/FMI. Audit actual harness consumers first; retain subprocess/GPU/MPI/memory/end-to-end qualification and machine-readable receipts when Criterion is not equivalent. Do not replace correct specialist harnesses merely for uniformity. This work can start early and supports per-track cost gates; it is not a global prerequisite.

## Acceptance criteria

Known injected regression is detected on a controlled host; repeated baselines, warmup/sample configuration and input hashes recorded; setup excluded appropriately, results consumed to prevent dead-code elimination, output metric/receipt parity, noise-aware thresholds. Report CPU affinity/power/toolchain where relevant; no claim that statistical significance proves user value.

## Quality gates

`dependency-101-behaviour`, `dependency-value-proof`, `phase-closeout-check`. Independent failure controls, full costs, exact version/maintenance/security/licence/MSRV/target qualification before adoption.

## CareOps handoff

Accepted immutable upstream receipts before a parent pin/API change. Parent owns its consumer and test packets. No schema or scientific policy invented upstream.

## Release implications

Public protocol/numerical changes require compatibility/version review; no hosted or operational acceptance from planning.

## Reference to requalify

https://bheisler.github.io/criterion.rs/book/

## CareOps-Sim impact and opportunity review

Closeout includes the [lightweight downstream review](../../dependency-value-policy.md#careops-sim-impact-and-opportunity-review). Kairos supplies its accepted immutable receipt; the CareOps-Sim coordinator and consumer owner record implications, worthwhile opportunities and one of the four outcomes in `conductor/dependency-handoffs.json`, keyed by `owner_track: 101-criterion-consolidation`. Reuse existing consumer tracks; no automatic direct dependency or duplicate implementation track. For already accepted phases this is downstream follow-through, not reopening or changing historical acceptance. Review remains pending until evidenced separately from implementation.

# Test matrix

All runtime gates are future requirements, not implemented or executed tests. At entry, create exact executable targets, fixture hashes and baseline-derived budgets before replacement.

| Gate | Required oracle |
| --- | --- |
| routing-deterministic-paths | Independent small-graph optimality, cost/hops/edge-ID ties, shuffled insertions, zero-cost cycles, directionality and overflow. |
| routing-multifloor-access | Synthetic multi-floor connectivity, inaccessible lifts/stairs/doors, disconnected destination, topology revision/cache invalidation and unit/identity rejection. |
| routing-des-boundary | Lift queues/resource waits remain authoritative DES state; route rendering does not schedule events; code/build/query-memory audit without fictitious deletion claims. |

Planning: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`. Runtime additionally requires `just ci` and relevant consumers. Hosted configuration uses `just quality-drift`; unavailable is not pass.

Runtime: exact named native/browser tests and independent oracles defined at entry; record command, cwd, source commit, toolchain, resolved versions/features, seed/input hashes, exit status and raw artifact path. No source-text assertions substitute for actual browser/runtime behaviour.

Closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`; clean-tree verification and review are required before advancing.

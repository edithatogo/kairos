# Test matrix

All runtime gates are future requirements, not implemented or executed tests. At entry, create exact executable targets, fixture hashes and baseline-derived budgets before replacement.

| Gate | Required oracle |
| --- | --- |
| sampling-independent-oracles | Analytical moments/CDF or quantile controls with fixed seeds, sample sizes and justified error bounds; tails, support, invalid/degenerate and bounded-integer cases. |
| sampling-hazard-parity | Independent exponential/piecewise survival and cause incidence, unchanged-covariate residual clocks, censoring and no hidden clamping; reviewed corrections separately identified. |
| sampling-resume-cost | Restart/worker equality on qualified targets, draw isolation and versioned manifests; actual production deletion and throughput/build cost. |

Planning: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`. Runtime additionally requires `just ci` and relevant consumers. Hosted configuration uses `just quality-drift`; unavailable is not pass.

Runtime: exact named native/browser tests and independent oracles defined at entry; record command, cwd, source commit, toolchain, resolved versions/features, seed/input hashes, exit status and raw artifact path. No source-text assertions substitute for actual browser/runtime behaviour.

Closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`; clean-tree verification and review are required before advancing.

# Test matrix

All runtime gates are future requirements, not implemented or executed tests. At entry, create exact executable targets, fixture hashes and baseline-derived budgets before replacement.

| Gate | Required oracle |
| --- | --- |
| rng-versioned-replay | Pinned generator known-answer vectors, full entity generations and purpose namespaces, intended CRN coupling, unrelated-draw isolation and 1/2/4-worker exact traces. |
| rng-checkpoint-compatibility | Roundtrip before/after partial-buffer draws and nonzero stream positions; uninterrupted versus resumed exact draws/events; malformed, old and unknown versions reject. |
| rng-dependency-cost | Reviewed MSRV/target matrix including Wasm, no accidental entropy source; build/binary and per-draw/whole-run cost; bespoke-generator deletion audit. |

Planning: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`. Runtime additionally requires `just ci` and relevant consumers. Hosted configuration uses `just quality-drift`; unavailable is not pass.

Runtime: exact named native/browser tests and independent oracles defined at entry; record command, cwd, source commit, toolchain, resolved versions/features, seed/input hashes, exit status and raw artifact path. No source-text assertions substitute for actual browser/runtime behaviour.

Closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`; clean-tree verification and review are required before advancing.

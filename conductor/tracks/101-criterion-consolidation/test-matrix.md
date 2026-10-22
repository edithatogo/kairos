# Test matrix

Future work, no runtime acceptance.

| Gate | Oracle |
| --- | --- |
| dependency-101-behaviour | Known injected regression is detected on a controlled host; repeated baselines, warmup/sample configuration and input hashes recorded; setup excluded appropriately, results consumed to prevent dead-code elimination, output metric/receipt parity, noise-aware thresholds. Report CPU affinity/power/toolchain where relevant; no claim that statistical significance proves user value. |
| dependency-value-proof | Demonstrated consumer, net reduction/required capability, exact features/maintenance/security/licence/target and bounded costs |
| phase-closeout-check | Review/fixes and exact command/cwd/commit/toolchain/input/seed/exit/artifact receipts |

Run focused native tests and `just ci` for implementation; planning validators cover gates/DAG/artifacts only.

Planning: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`. Closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.

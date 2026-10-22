# Test matrix

Future work, no runtime acceptance.

| Gate | Oracle |
| --- | --- |
| dependency-102-behaviour | Analytical reference solutions and expected convergence order; tolerance refinement and independent reference solver; conservation/positivity without hiding error by clamping; discontinuity/event-boundary fixtures, determinism on supported targets and bounded runtime. Qualify maintenance, platform support and needed solver class before selecting a release. |
| dependency-value-proof | Demonstrated consumer, net reduction/required capability, exact features/maintenance/security/licence/target and bounded costs |
| phase-closeout-check | Review/fixes and exact command/cwd/commit/toolchain/input/seed/exit/artifact receipts |

Run focused native tests and `just ci` for implementation; planning validators cover gates/DAG/artifacts only.

Planning: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`. Closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.

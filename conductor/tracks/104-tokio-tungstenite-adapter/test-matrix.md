# Test matrix

Future work, no runtime acceptance.

| Gate | Oracle |
| --- | --- |
| dependency-104-behaviour | Cancellation-safe connection lifecycle, bounded queues/backpressure, shutdown/reconnect, slow consumers, no lost/duplicate command effects and same protocol conformance as Track 103. Measure actual benefit over accepted simpler transport and count transitive/build cost. |
| dependency-value-proof | Demonstrated consumer, net reduction/required capability, exact features/maintenance/security/licence/target and bounded costs |
| phase-closeout-check | Review/fixes and exact command/cwd/commit/toolchain/input/seed/exit/artifact receipts |

Run focused native tests and `just ci` for implementation; planning validators cover gates/DAG/artifacts only.

Planning: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`. Closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.

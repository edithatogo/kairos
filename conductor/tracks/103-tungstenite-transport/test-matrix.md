# Test matrix

Future work, no runtime acceptance.

| Gate | Oracle |
| --- | --- |
| dependency-103-behaviour | Real loopback browser/native interoperability; malformed/oversize/fragmented messages, close/ping/pong, slow-reader backpressure, reconnect/gaps, cancellation and no duplicate commands/arrivals. Explicit origin/auth/TLS boundary appropriate to deployment; no production service implied. Test renderer-off identical simulation outcomes; finite resource limits. |
| dependency-value-proof | Demonstrated consumer, net reduction/required capability, exact features/maintenance/security/licence/target and bounded costs |
| phase-closeout-check | Review/fixes and exact command/cwd/commit/toolchain/input/seed/exit/artifact receipts |

Run focused native tests and `just ci` for implementation; planning validators cover gates/DAG/artifacts only.

Planning: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`. Closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.

# Test matrix

Future tests; none implemented or run by registration.

| Gate | Oracle |
| --- | --- |
| dependency-82-behaviour | Real process shutdown/cancellation, slow consumers, saturation, reconnect and bounded resources; deterministic handoff ordering despite different IO timing. |
| dependency-value-proof | Named production target/accepted consumer; independent regression/negative tests; exact pins/features/licence/security/MSRV; wrapper-inclusive code reduction or necessary avoided implementation; build/binary/memory/latency budgets fixed before replacement |
| phase-closeout-check | Conductor review/fixes, exact receipts, clean claimed paths and no missing runtime oracle |

Planning validators: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`, `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`. Runtime: `just ci` plus concrete independent/native consumers defined in Phase 0. `just quality-drift` is read-only hosted configuration evidence; preserve unavailable/pending/drift outcomes.

Closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`. Record command, cwd, commit, toolchain, resolved versions/features, seed/input hashes, exit status and raw artifact path; no broad green test substitutes for a missing behavioural oracle.

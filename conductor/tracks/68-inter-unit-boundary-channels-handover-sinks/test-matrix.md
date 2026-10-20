# Test matrix: composition refinement

| Phase | Required oracle |
| --- | --- |
| 0 | Freeze neutral operation IDs/envelopes and requested/offered/reserved/departing/in-transit/arrived/acknowledged/rejected/cancelled/expired states, including failure and compensation transitions. Write failing two-depot fixtures for bounded capacity, sender holds, destination reservations and authority/resource-release points; an accepted message is not completed transfer. |
| 1 | Implement deterministic bounded transfers on the existing scheduler first; telemetry streaming and distributed transport are optional adapters, not the state authority. Implement retry idempotency, expiry, delayed/duplicate acknowledgement, cancellation before departure and fallback after departure; distinguish reserved capacity from physical occupancy. |
| 2 | Verify exactly-once logical effects under repeated delivery, no lost/duplicated entities, horizon censoring and no premature origin/destination/escort release. Qualify checkpoint/resume in every transit state and consumer rejection paths. Coordinate Arrow schema work with 04/62; real process/transport faults require separate 34/35 qualification. |
| 3 | Owner-native tests, compatibility, exact-source phase/DAG and hosted evidence; no promotion from a planned command. |

Resolve real executable targets at dispatch; future commands are not test results.
Unavailable dependencies remain explicit and do not pass through mocks.

## Conductor closeout checks

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`

The strict clean-tree closeout gate applies to accepted implementation phases.
Planning validation does not establish a passed runtime or publication gate.

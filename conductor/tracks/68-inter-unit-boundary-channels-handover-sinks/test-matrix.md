# Test matrix: composition refinement

| Phase | Required oracle |
| --- | --- |
| 0 | Freeze neutral operation IDs/envelopes and requested/offered/reserved/departing/in-transit/arrived/acknowledged/rejected/cancelled/expired states, including failure and compensation transitions. Write failing two-depot fixtures for bounded capacity, sender holds, destination reservations and authority/resource-release points; an accepted message is not completed transfer. |
| 1 | Implement deterministic bounded transfers on the existing scheduler first; telemetry streaming and distributed transport are optional adapters, not the state authority. Implement retry idempotency, expiry, delayed/duplicate acknowledgement, cancellation before departure and fallback after departure; distinguish reserved capacity from physical occupancy. |
| 2 | Verify exactly-once logical effects under repeated delivery, no lost/duplicated entities, horizon censoring and no premature origin/destination/escort release. Qualify checkpoint/resume in every transit state and consumer rejection paths. Coordinate Arrow schema work with 04/62; real process/transport faults require separate 34/35 qualification. |
| 3 | Owner-native tests, compatibility, exact-source phase/DAG and hosted evidence; no promotion from a planned command. |

Resolve real executable targets at dispatch; future commands are not test results.
Unavailable dependencies remain explicit and do not pass through mocks.

## Registered acceptance gates

All executable bindings below are **pending**. Before dispatch, the owner must
bind a real test target, immutable source/features, fixture and seed, toolchain,
and retained output hash. These rows define acceptance, not executed results.

| Gate ID | Required oracle | Executable binding / status |
| --- | --- | --- |
| `phase-closeout-check` | Run the Conductor closeout checks below; implementation acceptance additionally requires runtime receipts and independent review. | Document commands listed below; implementation closeout pending |
| `boundary-channel-transit-delay-distribution` | Fixed-clock and seeded transit cases preserve units, tie order, bounded delay semantics and independently specified distribution controls; unsupported distributions remain unqualified | Pending owner-native executable binding; no runtime pass |
| `egress-sink-handover-backpressure-parity` | Reservation/occupancy/authority transitions preserve sender holds and destination capacity under rejection, expiry, cancellation and delayed acknowledgements | Pending owner-native executable binding; no runtime pass |
| `inter-unit-entity-conservation-accounting` | Single-scheduler two-depot fixture proves one logical effect per operation ID across duplicates/reordering/restart; lost/duplicated entities or premature resource release fail, without claiming network exactly-once delivery | Pending owner-native executable binding; no runtime pass |

## Conductor closeout checks

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`

The strict clean-tree closeout gate applies to accepted implementation phases.
Planning validation does not establish a passed runtime or publication gate.

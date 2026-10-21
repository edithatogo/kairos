# Test matrix: composition refinement

| Phase | Required oracle |
| --- | --- |
| 0 | Inventory Track 59/01 relationship APIs and review neutral placement without forcing game-theory solver dependencies. Freeze one-to-many cardinality, typed edges, generational handles and stable external identity mappings. Write failing fixtures for deterministic spawn during event handling, multiple children, stale handles, duplicate creation and explicit independent/coupled lifecycle policy. Reserve only the reviewed existing owner paths. |
| 1 | Implement the smallest accepted neutral relationship/spawn slice with safe iteration and explicit capacity/budget failures; do not require unbounded allocation or assume every dynamic spawn is allocation-free. Implement explicitly selected detach/reparent/cascade semantics. Parent termination must not implicitly delete independent children; preserve deterministic cleanup and accounting. |
| 2 | Coordinate schema and Arrow implementation with 04/62 rather than writing unowned IO paths. Support one-to-many links, distinct lifecycle events and versioned external identity mappings. Qualify repeated create/terminate/recover, parent/child spatial separation, checkpoint round trip and unchanged downstream ED traces; clinical birth/episode semantics remain downstream. |
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
| `dynamic-entity-spawning-ecs-integrity` | Safe mid-iteration spawn, deterministic IDs and bounded capacity; reject stale handles, duplicate creation and allocation failure without corrupted iteration | Pending owner-native executable binding; no runtime pass |
| `typed-relationship-lifecycle-parity` | One-to-many typed links with explicit detach/reparent/cascade rules; independent children survive, ownership cycles reject, and checkpoint/replay conserves identities | Pending owner-native executable binding; no runtime pass |
| `composite-entity-telemetry-arrow-export` | Versioned relationship IDs survive real Arrow IPC/Parquet export and independent-reader round trips; duplicate/missing links and incompatible schemas fail | Pending owner-native executable binding; no runtime pass |

## Conductor closeout checks

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1`
- `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`

The strict clean-tree closeout gate applies to accepted implementation phases.
Planning validation does not establish a passed runtime or publication gate.

# Conformance Fixtures

This directory holds the bootstrap fixtures used by Track 12 and consumed by Track 01, Track 02, and Tracks 06-11.

The target directory-backed runner shape remains the one in `conductor/contracts/conformance-contract.md`. The files listed here are the current bootstrap layer that downstream tracks can consume now.

## Canonical fixture families

| Fixture ID | Current file | Purpose | Consumers |
|---|---|---|---|
| `scheduler_ordering_v1` | `deterministic_ordering.json` | Deterministic ordering by `time`, `priority`, and `sequence` | Track 01, Track 02, Tracks 06-11 |
| `scheduler_cancellation_v1` | `cancellation.json` | Cancellation without reordering the remaining queue | Track 01, Track 02, Tracks 06-11 |
| `rng_reproducibility_v1` | `rng_replay.json` | Reproducible entity-derived random streams | Track 01, Track 02, Tracks 06-11 |
| `vvuq_scenario_replay_v1` | `vvuq_scenario_replay.json` | Scenario/seed replay evidence boundary for the local VVUQ smoke | Tracks 21-22 |
| `zero_delay_guard_v1` | planned | Guardrail for zero-delay livelock behavior | Track 01, Track 02, Tracks 06-11 |
| `des_resource_queue_v1` | planned | Resource queue behavior for DES workflows | Track 01, Track 02, Tracks 06-11 |
| `abm_behavior_update_v1` | planned | Behavior update semantics for ABM workflows | Track 01, Track 02, Tracks 06-11 |
| `hybrid_des_abm_v1` | planned | Mixed DES and ABM run contract | Track 01, Track 02, Tracks 06-11 |
| `arrow_event_log_v1` | planned | Arrow event-log fingerprint compatibility | Track 04, Track 01, Track 02, Tracks 06-11 |
| `ffi_lifecycle_v1` | planned | Handle lifecycle and ownership safety across FFI | Track 02, Tracks 06-11 |

## Shared fields

Ready bootstrap fixtures must keep the following fields stable:

- `fixture`
- `version`
- `events` when the fixture exercises scheduler order
- `expected_kind_order` when the fixture asserts dispatched order
- `run_seed` and `entity` when the fixture exercises reproducible RNG
- `scenario_manifest`, `seed_manifest`, and `expected_summary_hash` when the fixture exercises VVUQ replay evidence
- `requirement` when the fixture carries a plain-language contract note

## Consumption rules

- Track 01 owns the core behavioral meaning of the ready fixtures.
- Track 02 consumes the same meaning through the stable facade.
- Tracks 06-11 must not rename fixture IDs or restate the assertions in divergent local formats.
- Track 12 owns the manifest and benchmark naming, not per-binding reinterpretations.

# Track 01 Implementation Lanes

Track 01 is too large to run as a single undifferentiated task. Use these lanes for ownership, review, and handoff.

| Lane | Owner | Paths | First acceptance gate |
|---|---|---|---|
| 01A Types and time | contracts-agent | `crates/kairo-ecs-types/` | `SimTime`, durations, IDs, and scheduler DTOs compile and test. |
| 01B Scheduler | core-scheduler-agent | `crates/kairo-ecs-core/` | Deterministic ordering and cancellation tests pass. |
| 01C State | ecs-agent | `crates/kairo-ecs-state/` | Entity lifecycle tests pass and storage ADR is drafted. |
| 01D RNG | rng-agent | `crates/kairo-ecs-rng/` | Entity-derived stream replay test passes. |
| 01E Facade readiness | core-scheduler-agent + ffi-agent | `crates/kairo-ecs-core/`, `conductor/contracts/` | Public Rust facade maps cleanly to handles and status codes. |

## Promotion rule

Do not start binding implementation from Track 01 alone. Binding agents need:

- Track 01 lanes 01A, 01B, and 01E accepted.
- Track 02 FFI contract accepted.
- Track 12 conformance fixture runner available.


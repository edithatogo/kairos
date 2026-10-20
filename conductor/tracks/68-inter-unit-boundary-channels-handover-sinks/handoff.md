# Track 68 Handoff: Inter-Unit Boundary Channels & Asynchronous Handover Sinks

Status: Planned

Freshness: 2026-10-20

## Summary

Track proposed to introduce SimPy/DES asynchronous boundary channels and decoupled egress sinks in `crates/kairo-ecs-des`.

## Files changed

- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/tracks/68-inter-unit-boundary-channels-handover-sinks/*`

## Contracts consumed

- `crates/kairo-ecs-des` process, queue, and resource interfaces.

## Contracts changed

- Added `BoundaryChannel` and `EgressSink` in `kairo-ecs-des`.

## Tests added

- Proposed in `test-matrix.md`.

## Known risks

- Channel buffer capacity limits; see `risk-register.md`.

## Follow-up issues

- None currently identified.

## Integration notes

- Directly consumed by `careops-core` and downstream clinical tracks (`careops-birthing`, `careops-surgery`) for inter-unit handovers.

# Track 48: Time Warp Optimistic Rollback Runtime

## Purpose

Implement an optimistic PDES runtime with Time Warp rollback, anti-messages,
state saving, fossil collection, and generational component bitsets.

## Maturity

Spec Approved planning track. No production Time Warp rollback runtime is
claimed by this artifact.

## Inputs

- Track 47 LP, event, and safe-time contracts.
- ECS state model from Track 01.
- Time-travel/debug concepts from Track 40.
- HPC evidence manifest from Track 46.

## Outputs

- `time-warp` feature-gated optimistic runtime in `crates/kairo-ecs-pdes/`.
- State-save and rollback APIs with generation-aware component validity.
- Anti-message routing and cancellation semantics.
- Fossil collection tied to GVT.
- Rollback stress tests and overhead benchmarks.

## Owned paths

- `crates/kairo-ecs-pdes/`
- `docs/pdes/`
- `benches/pdes/`
- `conductor/tracks/48-time-warp-optimistic-rollback-runtime/`

## Blocked paths

- `crates/kairo-ecs-state/` without Track 01/ecs-agent handoff.
- `crates/kairo-ecs-debug/` without Track 40 handoff.
- Distributed transport code owned by Track 49.

## Dependencies

Track 47.

## Parallel-safe tracks

Track 49 may draft distributed rollback transport tests after anti-message
wire requirements are documented.

## Acceptance criteria

- Causality violations trigger rollback to the correct prior state.
- Anti-messages cancel matching positive messages deterministically.
- Fossil collection never removes state needed by a later rollback.
- Generational bitsets prevent stale component access after rollback.
- Optimistic runtime reports rollback rate, memory pressure, and GVT lag.

## Quality gates

- `time-warp-rollback-parity`
- `time-warp-antimessage-integrity`
- `generational-bitset-rollback`
- `fossil-collection-safety`
- `hpc-evidence-manifest`
- `phase-closeout-check`

## Release implications

This track is release-critical for any optimistic PDES or Time Warp claim. It
must remain opt-in behind `time-warp`.

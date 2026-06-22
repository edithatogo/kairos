# Track 47 Handoff

Last updated: 2026-06-22

## Summary

Track 47 is the production conservative PDES runtime track. The current local
implementation slice adds typed conservative lookahead enforcement, deterministic
LP partition planning, local sequential/partitioned parity evidence, GVT
monotonicity checks, deadlock smoke evidence, and 4/8/16/32 LP local
benchmark-smoke samples without changing blocked core scheduler internals.

## Files changed

- `crates/kairo-ecs-pdes/src/lib.rs`
- `docs/pdes/gvt-algorithm.md`
- `docs/pdes/validation-evidence.md`
- `conductor/tracks/47-pdes-conservative-lookahead-production-runtime/*`

## Contracts consumed

- Track 34 LP and GVT scaffold.
- Track 46 evidence manifest.
- Track 31 benchmark regression policy.

## Contracts changed

The local scheduler now rejects remote events earlier than
`lp.local_time() + lp.lookahead()` with `PdesError::LookaheadViolation`.
`PartitionPlan::from_entities` provides deterministic entity-to-LP assignment
and rejects invalid partition inputs before runtime startup.
Tracks 49 and 55 should treat this as the conservative safe-time boundary for
future distributed transport and scaling evidence.

## Tests added

- `scheduler_rejects_remote_events_before_declared_lookahead`
- `partition_plan_assigns_entities_deterministically_by_entity_id`
- `partition_plan_rejects_invalid_inputs`

## Known risks

The current evidence remains local and dependency-free. Full production
conservative scheduler integration, wall-clock benchmark targets, Track 46
raw evidence manifests, live scaling proof, and hardware throughput claims
remain unimplemented.

## Follow-up issues

- Complete production conservative scheduler integration beyond the local
  reference/scaffold evidence.
- Add controlled wall-clock benchmark targets and raw result artifacts.
- Record live scaling evidence.

## Integration notes

Track 49 must not assume distributed transport semantics until this track hands
off the production LP contract.

## Phase closeout evidence

Run `$conductor-review`, record accepted fixes, commit SHA, pushed ref,
`validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and the
next-phase decision before advancing this track.

Latest local commands:

- `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features pdes`
- `rustup run stable-x86_64-pc-windows-gnu cargo fmt --check -p kairo-ecs-pdes`

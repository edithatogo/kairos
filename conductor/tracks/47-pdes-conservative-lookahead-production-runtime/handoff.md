# Track 47 Handoff

Last updated: 2026-06-19

## Summary

Track 47 is the production conservative PDES runtime track. The first
implementation slice adds typed conservative lookahead enforcement to the local
PDES scheduler without changing blocked core scheduler internals.

## Files changed

- `crates/kairo-ecs-pdes/src/lib.rs`
- `conductor/tracks/47-pdes-conservative-lookahead-production-runtime/*`

## Contracts consumed

- Track 34 LP and GVT scaffold.
- Track 46 evidence manifest.
- Track 31 benchmark regression policy.

## Contracts changed

The local scheduler now rejects remote events earlier than
`lp.local_time() + lp.lookahead()` with `PdesError::LookaheadViolation`.
Tracks 49 and 55 should treat this as the conservative safe-time boundary for
future distributed transport and scaling evidence.

## Tests added

- `scheduler_rejects_remote_events_before_declared_lookahead`

## Known risks

Sequential parity fixtures, full LP partitioning, GVT/deadlock stress beyond
the existing scaffold, benchmarks, and live scaling proof remain unimplemented.

## Follow-up issues

- Implement the feature-gated conservative scheduler.
- Add failing parity, GVT, and deadlock tests for the remaining production
  runtime behavior.
- Record live scaling evidence.

## Integration notes

Track 49 must not assume distributed transport semantics until this track hands
off the production LP contract.

## Phase closeout evidence

Run `$conductor-review`, record accepted fixes, commit SHA, pushed ref,
`validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and the
next-phase decision before advancing this track.

# Track 48 Handoff

Last updated: 2026-06-22

## Summary

Track 48 owns optimistic PDES and Time Warp rollback. The first implementation
slice adds a `time-warp` feature-gated local runtime primitive for straggler
rollback, anti-message cancellation, invalidated-send anti-message emission,
generation-checked component access, monotonic-GVT fossil collection, pre-GVT
straggler rejection, duplicate-positive rejection, and local overhead/rollback
pressure counters.

## Files changed

- `crates/kairo-ecs-pdes/Cargo.toml`
- `crates/kairo-ecs-pdes/src/lib.rs`
- `conductor/tracks/48-time-warp-optimistic-rollback-runtime/*`

## Contracts consumed

- Track 47 production LP contract.
- Track 40 trace/replay semantics.
- Track 46 evidence manifest.

## Contracts changed

The local `TimeWarpRuntime` now defines:

- `TimeWarpEventId` as the stable positive/anti-message match key.
- `TimeWarpEvent` with `Positive` and `Anti` message kinds.
- `TimeWarpStepReport` with rollback, canceled-event, and anti-message output.
- `TimeWarpComponentToken` generation checks for stale component access.
- `TimeWarpRuntime::fossil_collect(gvt)` with strict monotonic GVT advancement
  and pruning only for executed positive history older than GVT.
- `TimeWarpRuntime::overhead_metrics()` for local log, component generation,
  rollback, anti-message, duplicate-positive, and fossil-collection counters.
- `TimeWarpError` variants for stale generations, duplicate positives, GVT
  regression, and positive/anti-message arrivals older than GVT.

## Tests added

- `time_warp_straggler_rolls_back_to_prior_checkpoint`
- `time_warp_antimessage_cancels_matching_positive_event`
- `time_warp_generation_token_rejects_stale_component_access`
- `time_warp_fossil_collects_only_history_before_gvt`
- `time_warp_fossil_collection_preserves_rollback_at_or_after_gvt`
- `time_warp_rejects_pre_gvt_stragglers_and_gvt_regression`
- `time_warp_rejects_duplicate_positive_events_without_double_apply`
- `time_warp_overhead_metrics_track_local_rollback_pressure`

## Known risks

This is a local, dependency-free runtime primitive, not a full distributed Time
Warp scheduler. The current rollback model undoes future positives and emits
anti-messages for invalidated local inputs; it does not yet preserve a replay
queue for rolled-back future work or model downstream output anti-messages.
Optimistic execution beyond conservative safe time, benchmark evidence, live
HPC evidence, and distributed anti-message transport remain unimplemented.

## Follow-up issues

- Add replay-queue semantics for rolled-back future positives, or document the
  required Track 49 redelivery contract before any production Time Warp claim.
- Model downstream output anti-messages separately from canceled input events.
- Add rollback overhead benchmarks.

## Integration notes

Any ECS storage changes require ecs-agent handoff before implementation. Track
49 can draft transport tests around `TimeWarpEventId` and `TimeWarpMessageKind`,
but distributed delivery remains out of scope for this slice.

## Phase closeout evidence

Red step captured with
`rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features time-warp`;
the first run failed because `TimeWarpRuntime`, event, anti-message, and
generation token types did not exist.

Passing implementation gates:

- `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features time-warp time_warp`
- `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features time-warp`
- `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features pdes`
- `rustup run stable-x86_64-pc-windows-gnu cargo check --benches -p kairo-ecs-pdes --features time-warp`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo clippy -p kairo-ecs-pdes --all-targets --all-features -- -D warnings`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features --jobs 1`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo clippy --workspace --all-targets --all-features --jobs 1 -- -D warnings`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_dag.ps1`

Initial full-workspace attempts without `--jobs 1` failed with OS error 112,
"There is not enough space on the disk." Removing generated build output and
rerunning with `CARGO_INCREMENTAL=0` and serialized jobs completed cleanly.

Implementation commit SHA: `35f93a4344615e2f8a4e5ca8a61ad7a483e87106`
pushed ref: `origin/codex/kairos-hpc-parity-wave`

2026-06-22 `$conductor-review` implementation pass for the fossil/GVT slice
recorded accepted fixes before closeout: explicit pre-GVT positive and
anti-message rejection, monotonic GVT regression errors, duplicate-positive
rejection, and bounded metrics for local rollback pressure. The review also
records replay-queue and downstream-output anti-message semantics as follow-up
risks rather than completed production Time Warp behavior. Strict closeout will
run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit
and push. The next-phase decision is to keep Track 48 In Progress until
optimistic safe-time execution, replay/redelivery semantics, benchmark evidence,
distributed transport, and live HPC evidence are complete.

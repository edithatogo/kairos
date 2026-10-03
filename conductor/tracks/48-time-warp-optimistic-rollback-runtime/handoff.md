# Track 48 Handoff

Last updated: 2026-06-24

## Summary

Track 48 owns optimistic PDES and Time Warp rollback. The first implementation
slice adds a `time-warp` feature-gated local runtime primitive for straggler
rollback, anti-message cancellation, invalidated-send anti-message emission,
generation-checked component access, monotonic-GVT fossil collection, pre-GVT
straggler rejection, duplicate-positive rejection, and local overhead/rollback
pressure counters. The 2026-06-24 evidence-worker pass adds a local scaffold
manifest for that proof boundary without expanding the runtime claim to
distributed optimistic rollback.

## Files changed

- `crates/kairo-ecs-pdes/Cargo.toml`
- `crates/kairo-ecs-pdes/src/lib.rs`
- `conductor/tracks/48-time-warp-optimistic-rollback-runtime/*`
- `conductor/hpc-evidence/manifests/track48-local-time-warp-rollback-scaffold.json`

## Contracts consumed

- Track 47 production LP contract.
- Track 40 trace/replay semantics.
- Track 46 evidence manifest.
- Track 49 distributed transport boundary, consumed only as a blocked handoff
  line for future distributed proof.

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
HPC evidence, and distributed anti-message transport remain unimplemented. The
local evidence manifest
`conductor/hpc-evidence/manifests/track48-local-time-warp-rollback-scaffold.json`
is scaffold evidence only: it records local rollback/GVT/anti-message contract
coverage and an explicit `not-live` waiver. It does not prove distributed
optimistic rollback, cross-rank causality repair, replay redelivery,
downstream-output anti-message propagation, or live HPC execution.

## Follow-up issues

- Add replay-queue semantics for rolled-back future positives, or document the
  required Track 49 redelivery contract before any production Time Warp claim.
- Model downstream output anti-messages separately from canceled input events.
- Add rollback overhead benchmarks.
- Replace the local scaffold manifest with live distributed rollback evidence
  that records transport implementation, MPI/gRPC rank topology, raw artifacts,
  checksums, and reviewer acceptance before any `evidence-backed` Time Warp
  release claim.

## Integration notes

Any ECS storage changes require ecs-agent handoff before implementation. Track
49 can draft transport tests around `TimeWarpEventId` and `TimeWarpMessageKind`,
but distributed delivery remains out of scope for this slice. A valid
distributed optimistic rollback proof must come from Track 48 plus Track 49
integration and include at least: straggler delivery across process or rank
boundaries, deterministic anti-message routing for invalidated downstream
outputs, replay/redelivery of rolled-back future positives, GVT advancement
across participants, raw run artifacts, and checksums. The local manifest gate
validates evidence shape and claim boundaries only.

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
- `node scripts/validation/validate-hpc-parity-evidence.mjs`

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


## Resumed implementation — after Track 47 merge

PR #193 merged as fc2f7b7f8e24faef8d02e29aaaaaf64852945cf0 after all 19 exact-head workflow runs passed. Active branch is codex/kairos-track48-optimistic-runtime; parent checkout/pin remain outside this task. Architecture and ECS/trace reviews settled the bound contract in docs/pdes/optimistic-runtime-contract.md. No distributed/Done claim is made.

Actual RED baseline at source fc2f7b7: Rust 1.98.1 compiled three regression tests and exited 101. Initial seed restoration, stale-token recreation and anti rebuild each fail their independent oracle. Test source is integrated from d1e6472 as 1b3c0d8. Worker logs/receipt remain in /private/tmp/kairos-track48-regressions/artifacts/track48-red-baseline. One advisory lease token was mistakenly printed during a check; it was released immediately after commit and root verified no such lease remains active. No token is committed.

GenerationBitset source eb8a15153ac95666a3962e895ea6bfbbeb738191fc4b7a83c763ed00ae725a90 integrated at b4be30d. Worker standalone 11 tests pass on Rust 1.98.1; independent reviewer also ran all 11 on Rust 1.76.0 and 1.98.1 with exit 0. Module exports/full crate integration remain pending. Construction/restore allocation paths are source-reviewed; allocation failure was not fault-injected. Snapshot cloning uses ordinary Vec clone; no blanket allocation-safe claim is made.

The original Track 48/49 distributed acceptance cycle remains explicit. No Track 49 production dispatch or dependency change is authorized by this local packet. Current EXC-193 is PR #193-only; any future exception needs its own governed approval.

## Independent legacy repair acceptance

Legacy repair integrated from a71b654 as fd29a8382992356dff481b9647ab3c2f8b205a1c. Five fixed-fixture regressions failed before the repair and pass afterward. Independent reviewer reran all five on Rust 1.98.1 and 1.76.0, both exit 0; exact command/source/toolchain/log hashes are in local artifacts/track48-legacy-review/receipt.json. lib.rs SHA-256 d6a152d413e81fe90f83bc58fa1f6601c35d7890e6ef1438fa0a396245ab5589; regression source a841c06bf8c5ac0dd8834de8639000c126c9af212a04bc9206a10cff41472788. Independent source review found no blocker in the focused repair.

Retained limitations: post-first-event component writes are not transactional initialization; checked stamp exhaustion panics under the legacy nonfallible API; diagnostic generations remain saturating; seed baselines/canceled IDs remain retained. The helper still uses tick-only ordering and input anti-history, without replay queues or downstream-send ownership. New production guarantees must come from the separately reviewed optimistic driver. A second worker check briefly surfaced eight token characters; its complete token was not saved and the lease is now released.

The proposed distributed-interface-handoff.md is a draft only. No scheduling dependency, Track49 production authority, phase status, or live evidence gate has changed.

## Integrated local optimistic driver — verification pending

Integrated worker0709f2f and independent test drafts c5ca802/d48ce2d/79d0cfe into 3cd6d56ed98d39d311fe19f4d88857f43bfe6da6. The driver owns replay queues, complete process snapshots, actual downstream send logs, exact source-scoped incarnation cancellation, structural ordering, checked validity epochs and bounded GVT/failure transitions. The bitset module is now exported under time-warp. This is local implementation, not distributed proof or a Done claim.

Worker exact-source native time-warp tests, Clippy and formatting pass on Rust1.98.1; native full time-warp tests also pass on Rust1.76.0. Fifteen reviewer-owned oracles passed in a temporary worker debug run, without source edits; private absolute includes were removed before commit. Independent post-integration rerun, benchmark, combined-feature/workspace and hosted gates remain pending. Imported log/receipt/source hashes are recorded locally in artifacts/track48-integration/worker-proof-hashes.json; original logs/receipt remain under /private/tmp/kairos-track48-optimistic/artifacts/track48-optimistic/.

Review fixes include global exact delivery metadata, legitimate changed replay metadata, preflighted batch epochs/capacities, canceled replay marker cleanup, and maximum-frontier GVT lag. Five private exhaustion/batch-transition tests supplement eight worker integration tests and fifteen independent state/RNG/bitset/replay/failure oracles. Recovery preserved the interrupted source in stash bbbf38f96f55a6c1dcf57af91f10ad0956eb5358; the restarted lease and final writer lease are released.

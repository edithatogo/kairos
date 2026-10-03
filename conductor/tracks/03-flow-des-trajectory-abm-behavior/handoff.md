# Handoff — 03 The Flow: DES Trajectory API & ABM Behavior API

## Summary

Track 03 now has a minimal R2 implementation slice for the DES trajectory API and ABM behavior API. The DES crate wraps the shared deterministic scheduler with a fixed-tick `TrajectoryRequest`/`Trajectory` request and trace surface. The ABM crate keeps the lightweight `ABMContext` component/scheduler facade and adds a deterministic `BehaviorSimulation`, `AgentBehavior`, `BehaviorContext`, and behavior-decision loop backed by per-agent RNG streams.

## Files changed

- `Cargo.toml`
- `Cargo.lock`
- `crates/kairo-ecs-des/Cargo.toml`
- `crates/kairo-ecs-des/src/lib.rs`
- `crates/kairo-ecs-des/tests/des_integration.rs`
- `crates/kairo-ecs-des/tests/des_resource_queue_v1.rs`
- `crates/kairo-ecs-abm/Cargo.toml`
- `crates/kairo-ecs-abm/src/lib.rs`
- `crates/kairo-ecs-abm/tests/abm_integration.rs`
- `crates/kairo-ecs-abm/tests/abm_behavior_update_v1.rs`
- `examples/flow/README.md`
- `conductor/tracks/03-flow-des-trajectory-abm-behavior/test-matrix.md`
- `conductor/tracks/03-flow-des-trajectory-abm-behavior/handoff.md`

## Contracts consumed

`conductor/workflow.md`, `conductor/contracts/core-contract.md`, and `conductor/contracts/conformance-contract.md`. The new APIs preserve fixed-tick time, scheduler ordering by `(time_ticks ASC, priority ASC, sequence ASC)`, generational entity handles, and deterministic fixture coverage for public behavior.

## Contracts changed

No shared contracts were changed for this track.

## Tests added

- `crates/kairo-ecs-des/src/lib.rs`: scheduler-order replay and bounded trajectory smoke tests.
- `crates/kairo-ecs-des/tests/des_integration.rs`: FIFO resource queue and fixed-tick scheduling smoke coverage.
- `crates/kairo-ecs-des/tests/des_resource_queue_v1.rs`: named DES resource queue fixture covering FIFO admission and fixed-tick trajectory replay ordering.
- `crates/kairo-ecs-abm/src/lib.rs`: scheduler-ordered behavior updates, event-budget behavior, deterministic entity-RNG replay, and despawn decisions.
- `crates/kairo-ecs-abm/tests/abm_integration.rs`: component attachment and multi-agent scheduling smoke coverage.
- `crates/kairo-ecs-abm/tests/abm_behavior_update_v1.rs`: named ABM behavior-update fixture covering scheduler order and deterministic per-agent RNG replay.

## Validation run

- `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-abm -p kairo-ecs-des` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des --test des_resource_queue_v1` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core` passed on 2026-05-08.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state` passed on 2026-05-08.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` passed on 2026-05-08.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` passed on 2026-05-08.
- `cargo fmt --all --check` was not rerun for this closeout because the working tree already contains unrelated local Conductor closeout edits; focused Track 03 formatting passed.

## 2026-05-08 fixture hardening

- Added `des_resource_queue_v1` and `abm_behavior_update_v1` named integration fixtures under Track 03-owned crate test paths.
- `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-abm -p kairo-ecs-des` passed.
- `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm` passed with 22 tests across ABM and DES unit, integration, and named fixture tests.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` passed.

## Known risks

The current APIs are intentionally minimal. They do not yet export shared conformance fixture files under `conformance/fixtures/des_resource_queue_v1`, `abm_behavior_update_v1`, or `hybrid_des_abm_v1`. The ABM update kind is currently a crate-local `EventKind::Custom` value and should be reconciled if Track 01 later introduces domain-specific event kinds.

## Integration notes

Next step: bind the DES and ABM smoke paths to shared conformance fixture files under `conformance/fixtures/`, then add richer resource/queue and agent-decision examples under `examples/flow/`.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
## Phase closeout evidence

2026-05-08 fixture-hardening closeout:

- `$conductor-review` result: the untracked DES/ABM fixture tests are in Track 03-owned paths and should be retained as implementation-slice hardening evidence.
- Accepted fixes: added the two named fixture tests and linked them to Track 03 handoff, test matrix, status, and phase-closeout evidence.
- Deferred or blocked fixes: shared fixture files under `conformance/fixtures/des_resource_queue_v1`, `conformance/fixtures/abm_behavior_update_v1`, and `conformance/fixtures/hybrid_des_abm_v1` remain future Track 03/12 integration work.
- Validation commands: `cargo +stable-x86_64-pc-windows-gnu fmt -p kairo-ecs-abm -p kairo-ecs-des --check`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm`, and `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Cleanup state: the only in-scope dirty files were the two new Track 03 tests and their Conductor evidence updates.
- Commit SHA / pushed ref: `5dd1937566898b2e028ac61dab1e9dd173e6d919` on `origin/main` is the current pushed base for this local closeout pass.
- Strict cleanup gate: run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after the commit and push.
- Next-phase decision: Track 03 is `In Review`; reviewer signoff is still required before moving the DES/ABM flow APIs to `Done`.

2026-05-08 review closeout:

- `$conductor-review` result: one documentation evidence gap was found in `examples/flow/README.md`; no DES or ABM behavioral findings were found.
- Accepted fixes: added the flow example maturity label, reproducibility commands, and expected output for the named DES/ABM fixture gates.
- Deferred or blocked fixes: shared JSON fixture exports and richer hybrid/model-zoo scenarios remain Track 12/23 follow-up work.
- Validation commands: `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-des -p kairo-ecs-abm`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des --test des_resource_queue_v1`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core`, `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state`, `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo`, `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo`, and `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Cleanup state: strict clean-tree closeout was not run because the shared worktree already contains unrelated staged and unstaged edits across other tracks.
- Commit SHA / pushed ref: `ee8c123e0a6dddd27986e7e657642190ee4f2560` on `origin/main` is the current base for this local review closeout pass.
- Next-phase decision: Track 03 is `Done`; future shared conformance fixture exports, hybrid scenarios, and model-zoo examples should be handled by Track 12/23 follow-up work.

2026-05-08 ABM despawn regression fix:

- `$conductor-review` result: one high-severity behavioral bug was found in `BehaviorSimulation::run_for`; a future queued update could still invoke behavior for an agent already despawned by an earlier update.
- Accepted fixes: skip non-live agents before resolving per-agent RNG streams and invoking behavior callbacks, and add a regression test for despawned-agent future events.
- Validation commands: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1` and `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm`.
- Next-phase decision: Track 03 remains `Done`.

## 2026-10-03 Q1 additive resource lifecycle review

Reviewed source commit: d8791ad981429f5b7d3c2238d0b034c04781488e on
codex/careops-resource-lifecycle. Independent read-only source review found no
Q1 acceptance blocker. Parent acceptance awaits exact-commit owner CI.

Coverage: opaque generational handles, authoritative ECS resource/request/work
state, typed owned context, ordered derived queue and active allocation indexes,
checked admission/counters, buffered manual release/capacity/removal/despawn,
canonical inspection, UInt32 lifecycle ordinals, standard Error/Display, and
missing-owner builder rejection before mutation. All owner claims are cancelled
before arbitration and owned work/context is removed in canonical order.
Seven related component/lease outputs were integrated by one coordinator writer;
intermediate interface drafts were never treated as runtime acceptance.

Executed at /private/tmp/careops-d2-main-acceptance-20261003/libs/kairos:
- cargo test --locked -p kairo-ecs-des: exit 0, 33 tests, current Rust/Cargo 1.98.1.
- cargo clippy --locked -p kairo-ecs-des --all-targets -- -D warnings: exit 0.
- rustup run 1.76.0 cargo test --locked -p kairo-ecs-des: exit 0, 33 tests.
- cargo deny --locked --workspace --all-features check: all four categories pass.
Logs retained in parent .artifacts/blocker-resolution/q1-final-native.log,
q1-final-clippy.log, q1-msrv-final.log and q1-policy-verified.log.
The seeded invariant grid uses 32 deterministic LCG seeds, 100 operations each;
no empirical input or RNG-core changes. Legacy FIFO tests remain unchanged.
Rust 1.76 evidence covers selected DES on macOS ARM64, not the whole workspace.

Dependency-policy repair: 37 internal path references in 18 manifests gained
exact target versions; Unicode-3.0 was added for unicode-ident's existing
compound licence. Ban/advisory/source restrictions retained. thiserror 2.0.20
was reviewed against the selected Rust floor and lockfile. Independent review
confirmed metadata-only effects outside the owned DES crate.
The initial policy repair packet had incomplete input hashes/unrelated steps.
Its original record was retained; a fresh all-input verification-only packet was
bound at a7e4249 and scope-specific checks were rerun. It is not represented as
an original correctly bound source dispatch.

Bound coordinator evidence packet: Q1.4.evidence.coordinator, task Q1.2 combines
the dependent Q1.2/Q1.3 review; prerequisites for parent Q1.4 remain pending.
Reviewed Q1 test/manual traces cover closed capacity, recycled owner, duplicate
release, stale events and failed commands without leaked claims.

Limits: this is experimental manual FIFO lifecycle. Priority/rekey/deadlines,
timed completion/preemption, hooks and persistent same-tick budgets belong to
Q2-Q4. Full-world per-dispatch staging cost remains a Q5 benchmark concern.
Trusted Rust context destructors are not panic-isolated. Portable codecs and
checkpoint/restore remain Track 22. No clinical validation or universal backend
support is claimed. No broad polyglot `just ci` execution is claimed.

## 2026-10-03 Q2 non-preemptive queue join

Reviewed source through 6b5176b. Priority ordering uses signed resource priority,
then original committed admission sequence. Checked buffered cancellation/rekey
supports pending, queued and active requests. Grant clears the waiting deadline;
terminal claims retain identity and resubmission uses a new request.
Deadline <= dispatch time expires before resource arbitration, in either token
insertion order. Only command-target resources participate in boundary expiry;
owner despawn targets all its nonterminal claim resources canonically. Independent
timeout rows survive rejected explicit rekey. Full arithmetic/preflight errors
still preserve staged ECS state. Deadline tokens use reserved 4002; commands4000.

Independent review found and corrected wrong timeout event kind and unrelated
resource expiry. No further non-preemptive source blocker was found. Generated
qualification uses 32 LCG seeds x100 mixed priority/deadline/cancel/rekey/capacity/
release operations, checking capacity, terminal uniqueness, queue/active membership
and original sequence. Explicit tests cover both release/growth deadline insertion
orders, reverse equal-priority commands, scheduler priority override, active lease
rekey, pending cancel/stale events, unrelated resource causality, and two-token
admission budget preflight without leaked entity/work association.

Executed in the active Kairos checkout: 43 DES tests pass on current Homebrew
Rust/Cargo 1.99.0; pinned rustup1.98.1 complete DES+Arrow regressions also pass.
Release and Rust1.76 DES suites passed before the final growth-only regression;
the added growth test passed separately in release and Rust1.76. Clippy all-targets
with -D warnings passed after the final regression. Logs: parent ignored
.artifacts/blocker-resolution/q2-native.log, q2-release.log, q2-msrv.log,
q2-growth-release.log, q2-growth-msrv.log, q2-clippy-final.log and
q2-c1-pinned-native.log. No empirical inputs/new engine RNG draws.
Timed completion, Suspend/Abort/Restart, notifications and persistent same-tick
budgets remain Q3-Q4. World staging performance remains Q5; no performance claim.
Parent Q2 acceptance still awaits exact final commit native owner CI.

Parallel C1 partial: Luna implemented pure already-resolved UTC arithmetic and
semantic exclusions; independent review corrected conflated missing timezone,
DST ambiguity and DST-gap reasons. Public errors use already-reviewed
thiserror2.0.20. Sixteen Arrow package tests pass with legacy smoke API unchanged.
No timezone parser, IPC/Parquet, empirical mapping or full C1 acceptance follows.
Worker receipt remains .artifacts/c1-temporal/result.json; coordinator fixes and
pinned verification supersede its initial output hashes. Manifests/lock remained
coordinator-owned. Full C1 tasks remain pending.

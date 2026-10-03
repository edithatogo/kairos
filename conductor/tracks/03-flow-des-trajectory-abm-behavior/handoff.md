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

## Experimental Q3 development extension — 2026-10-03

### Scope and task mapping

Historical Done describes the minimal DES/ABM slice. Qualified development source 42896037a7fac793f63d2f8867584f4faeb0b78e extends Track 03 implementation. It does not complete later Flow capabilities. Parent Q3 closeout remains pending; Q4 declarative Flow/ABM/telemetry and Q5 performance remain pending.

- Q3.1 maps to Phase 3 fixtures. Primary low duration = 10 ticks; urgent duration = 2 ticks, arrival = tick 3. Urgent ends at tick 5; low ends at tick 12 under Suspend, tick 15 under Restart, or aborts at tick 3. Secondary urgent duration = 3 ticks, arrival = tick 4: urgent ends at tick 7; low ends at ticks 13/17 or aborts at tick 4. Nested interruptions, capacity-two victim ties, eligibility exclusions, zero-duration work, completion at the interruption tick in both insertion orders and suspended cancellation are covered. Before implementation, primary/secondary/nested fixtures failed compilation with 11/11/61 missing-API errors; no tests executed. Immutable repair e936b706 strengthens the oracles.
- Q3.2 maps to Phase 3 experimental DES implementation: deterministic victims, atomic replacement, elapsed/remaining/busy accounting, typed owned context, attempt/execution revisions and cancellation. Root reviewed aggregate rollback, rejection-boundary preservation and strict stale-token identity. At 1455f762, 71 DES tests passed on each actual Rust 1.98.1 and 1.76 compiler; owner run 37118457361 passed both hosts.
- Q3.3 maps to Phase 3 continuation tests and Phase 4 owned tests: typed deferred handlers emit once, Restart reuses the stored duration and immutable initial-template factory, and Abort never resumes. Generator v1 uses six seeds × 32 cases × three strategies = 576 cases per targeted run. Independent accounting applies after every dispatch, including empty events advancing time. Exact preemption counts and all terminal reasons are checked. Actual Rust 1.98.1/1.76 targeted tests and owner run 37121692873 passed; both host logs confirm the property binary and named test executed.
- Q3.4 maps to scoped review and next-wave disposition. Root audited primary low busy totals = 10/13/3 ticks for Suspend/Restart/Abort; waiting = 2 ticks under Suspend/Restart and 0 ticks under Abort. Secondary low busy totals = 10/14/4 ticks; waiting = 3 ticks under Suspend/Restart and 0 ticks under Abort. Owned context/factory/revision review confirms no duration redraw. Root Track 01 review confirms unchanged core/state/RNG; Track 25 classifies enum/struct-literal changes as experimental-breaking, development-only, with migration and release hold. These are bounded internal reviews, not external maintainer signatures.

### Evidence and limits

Property source SHA256: d900dc3a9256794ea99822b2fd4f2362fb1bac9ac0ef802fca2ccaf7db71f904.
Local receipt SHA256: cfeff46576f39b53089b159060def379fc204712b599eec2224a71e9d891c7b2.
Hosted receipt SHA256: 7d3ae2a408cb19f89c55b3c19872c067a824f37061d4cbe03bbb6e6b8168f9ea.

Parent 8494e303 records actual receipt locations and hashes in conductor/evidence/q3-prerequisite-task-acceptance-20261003.json and conductor/evidence/q3-property-qualified-pin-20261003.md. Manual and owner reviews are in conductor/evidence/q3-qualified-runtime-pin-20261003.md and conductor/evidence/q3-owner01-track25-review-migration-20261003.md.

Coverage is bounded, not exhaustive; no shrinking or worker-count claim. Public FlowDispatch hides completion-token identity; the black-box model complements the existing private identity-injection test. Q4 persistent notification budget/general ingress and Q5 clone/retention costs remain pending. No release or stable-compatibility acceptance is implied.

### Governance review and gates

The conductor-review skill uses the root-approved alternate-layout handshake: root README.md links conductor/tracks.md, conductor/status.md and conductor/tracks.yaml; parent conductor/index.md provides routing. Child conductor/index.md and conductor/README.md are absent. Required product/technology/workflow/guidelines and all style guides exist and were reviewed. Unchanged native tests were not rerun.

Actual phase validator command from this governance checkout:

~~~text
/private/tmp/careops-q3-pwsh-7.6.6/.artifacts/pwsh/runtime/pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1
~~~

It passed using verified PowerShell 7.6.6, exit 0, zero errors and zero warnings. Strict clean-tree command remains pending until root approves a commit and nonforce push:

~~~text
/private/tmp/careops-q3-pwsh-7.6.6/.artifacts/pwsh/runtime/pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree
~~~

Logs remain outside this source checkout. After strict proof, a bounded metadata follow-up will record the actual command, commit/ref and receipt in place of pending language. The final committed/pushed metadata head must pass phase and strict validators again. Owner CI then qualifies that exact head before a new parent pin and Q3 closeout review. Parent Q4 cannot advance before these gates are accepted.

### Preserved historical Track 03 ledger record

Copied verbatim from the qualified predecessor; these commands/results are historical, not freshly rerun.

~~~yaml
  - track_id: "03"
    phase: "track-closeout"
    state: closed
    review_command: "$conductor-review with independent read-only source review"
    review_result: "Existing minimal track closeout retained; additive manual resource slice has no Q1 source blocker; Q2 non-preemptive join reviewed; 43 DES tests pass, release/MSRV qualification recorded; C1 temporal partial remains unaccepted. Exact-commit hosted owner CI required before parent acceptance."
    fixes_applied: true
    validation_commands:
      - "cargo test --locked -p kairo-ecs-des"
      - "cargo clippy --locked -p kairo-ecs-des --all-targets -- -D warnings"
      - "rustup run 1.76.0 cargo test --locked -p kairo-ecs-des"
      - "cargo deny --locked --workspace --all-features check"
      - "pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1"
    git_status: "Clean source commit; reviewed evidence pending commit and push"
    commit_sha: "6b5176b781b80267ab4a620d205a2a137fdce4f5"
    pushed_ref: "origin/codex/careops-resource-lifecycle"
    next_phase_decision: "Manual resource slice only; parent acceptance follows owner CI; Q2-Q4 remain pending."
~~~

# Test Matrix — 03 The Flow: DES Trajectory API & ABM Behavior API

## Required tests

- `cargo test -p kairo-ecs-core` to cover the scheduler and shared event model that DES and ABM both depend on.
- `cargo test -p kairo-ecs-state` to keep the state transition layer deterministic while the trajectory and behavior APIs are being defined.
- `cargo test -p kairo-ecs-des --test des_resource_queue_v1` for the named DES fixture gate.
- `cargo test -p kairo-ecs-abm --test abm_behavior_update_v1` for the named ABM fixture gate.
- `cargo fmt --all --check` before any handoff that touches Rust code.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` to keep the conductor setup consistent.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` to prove the track is still accounted for in the wave policy and registry.
- `cargo test --workspace` once the track starts adding concrete DES and ABM code paths.
- `cargo test -p kairo-ecs-des -p kairo-ecs-abm` for the Track 03 deterministic trajectory and behavior-update smoke fixtures.

## Current CI commands

```bash
cargo fmt --all --check
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
cargo test -p kairo-ecs-des -p kairo-ecs-abm
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
```

## 2026-05-08 validation notes

- Passed: `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-des -p kairo-ecs-abm`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des --test des_resource_queue_v1`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state`.
- Passed: `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo`.
- Passed: `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo`.
- Not rerun for this closeout: `cargo fmt --all --check`, because the working tree already contains unrelated local Conductor closeout edits. Focused Track 03 formatting passed.

## 2026-05-08 fixture-hardening validation

- Passed: `cargo +stable-x86_64-pc-windows-gnu fmt -p kairo-ecs-abm -p kairo-ecs-des --check`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm`, including `des_resource_queue_v1` and `abm_behavior_update_v1`.
- Passed: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Not claimed: shared fixture files under `conformance/fixtures/`; those remain follow-up work with Track 12 alignment.

## 2026-05-08 review closeout validation

- Passed: `cargo +stable-x86_64-pc-windows-gnu fmt --check -p kairo-ecs-des -p kairo-ecs-abm`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des --test des_resource_queue_v1`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-des -p kairo-ecs-abm`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-core`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-state`.
- Passed: `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo`.
- Passed: `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo`.
- Passed: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Not run: strict clean-tree git closeout, because unrelated staged and unstaged edits from other tracks are present in the shared worktree.

## 2026-05-08 ABM despawn regression validation

- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm --test abm_behavior_update_v1`.
- Passed: `cargo +stable-x86_64-pc-windows-gnu test -p kairo-ecs-abm`.
- Coverage added: a despawned agent with queued future updates no longer receives behavior callbacks after removal from world state.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.

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

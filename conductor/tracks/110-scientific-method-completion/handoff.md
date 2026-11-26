# Handoff

Updated 26 November 2026. Status **In Progress**.

## S15 staged allocator replay — 26 November 2026

The experimental debugger example drives the real `StagedResourceRuntime`
through `ReplaySession`. Each debugger wakeup dispatches one live allocator
action at its scheduled tick. Cloning the model restores the versioned
allocator checkpoint with fresh handle ownership. The model uses entity IDs
to bind handles to the restored allocator.
[Runnable Rust example](../../../crates/kairo-ecs-debug/examples/staged_allocator_replay.rs)

Two requests at tick 2 queue behind a two-unit lease. The normal path releases
that lease at tick 8 and admits both requests in order. Rewinding between the
tick-2 dispatches restores the exact allocator checkpoint. Replaying the
suffix reproduces the debugger dispatches and allocator checkpoint bytes.
The fork withdraws the head request at tick 4, admits the waiting tail, and
replays its edited timeline. The parent's allocator checkpoint is identical
before the fork and after both branch passes. Its queue then follows the
original tick-8 path.

Independent native replay review covered `763f039c4a5c9d3335f27207086ef8e9f1650472`.
It found that the original foreign-lease test could fail because a release
was already scheduled and that the edited branch ran only forward.
The corrected source `f5993fec162628997c78428e0359bd11bed118d4` tests an
earlier close with the foreign request handle and rewinds the edited fork.
The reviewer reran the corrected example and its two tests and accepted
both fixes.

From the workspace root, Rust and Cargo 1.99.1 on macOS ARM64 ran the
following commands with exit `0`. The test suite passed 83 cases.

```sh
cargo test --locked --offline -j 4 -p kairo-ecs-debug -p kairo-ecs-des \
    --features kairo-ecs-des/checkpoint-json
cargo run --locked --offline -j 4 -p kairo-ecs-debug --example staged_allocator_replay
cargo clippy --locked --offline -j 4 -p kairo-ecs-debug -p kairo-ecs-des \
    --all-targets --features kairo-ecs-des/checkpoint-json -- -D warnings
```

The executable reported both checks.

```text
Staged allocator rewind and replay passed.
Staged allocator fork isolation passed.
```

The example source SHA-256 is
`9ad48eb372963e19aa8b57276eb5271a15db2f64ac93741b560374efd52e5d49`.

## Bounded scientific and native qualification

Reused the exact numerical and native references for S01–S10, including the
receiving Paignia game controls, independent sensitivity and hazard results,
ONNX CPU and the installed Rust queue. Current docs and release evidence refer
to the same capability register. [Scientific register](../../scientific-completion.md) ·
[Generated capability register](../../capabilities.md)

The macOS ARM64 test run at `c5959a9ce92dbbc8991ce4f54480b281099a8e34`
passed 80 scoped Rust cases for Metal DES, conservative PDES, debugger,
TCP/Tonic, and DES/ABM resources. PyArrow 26.0.1 independently checked the
single retained dispatch from a rewound Rust session. S13 and S14 reuse the
named external FMU, native file and service-queue restart results.
[Commands, inputs, toolchain and output hashes](evidence/native-qualification-20261125.json) ·
[Raw Arrow stream](evidence/s15-retained-dispatches.arrow)

## Summary

Close every stated scientific/advanced method with a correct implementation and independent evidence, preserving explicit unsupported boundaries until qualification.

## Files changed

The debugger gained opt-in retained dispatch export through the existing Arrow
IPC writer. This track's plan, test matrix and evidence record bind the
supported numerical and native scopes to the corresponding source and run.

## Contracts consumed

AGENTS.md, existing core/FFI/conformance/versioning contracts, the approved [programme](../../frontier-programme-20261107.md), Tracks 01, 12, 21, 65, 66.

## Contracts changed

The programme establishes immediate Rust-only support; this track qualifies its own specific outcomes. No release/publication or downstream-pin acceptance follows from track registration.

## Tests added

The debugger CLI exercises eviction followed by rewind and rejects blank run
identities. The captured Arrow stream has an independent reader oracle. The
[test matrix](test-matrix.md) links the remaining executed and historical checks.

## Known risks

Method fixes change saved results. Preserve legacy algorithm identity for interpretation while retiring unsupported claims; use independent oracles instead of reproducing the implementation in tests.

## Follow-up issues

Qualify affected consumer versions against the versioned replay and method
contracts. Keep the wider device, distributed, FMI, storage, browser and hybrid
cases with the named owners and explicit scopes in the [plan](plan.md).

## Integration notes

Integration chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns this slice in
`/Users/doughnut/.codex/worktrees/kairos-scientific-evidence-closeout/kairos` on `codex/scientific-evidence-closeout-20261125`. The changes cover the debugger export,
scientific evidence and shared claim guard. The owned native run is complete.
Hosted integration checks bind the final tree to the reviewed source.

## Phase closeout evidence

The independent `$conductor-review` matched the source and fixture hashes,
logs, platform and Arrow bytes. The accepted fixes narrow the wording to
the tested protocol and movement cases and count the nested TCP peer once.
The tested runtime commit SHA is `c5959a9ce92dbbc8991ce4f54480b281099a8e34`.
The published commit SHA is `9b6921782e8c7fc9a82a7e3f1a6c0dd540c2ca60` on the pushed ref `origin/main`.
Its tree matches the tested PR tree.
`scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`
passed against the integrated source. The next-phase decision keeps Phase 0 open
for the remaining caller and version map and oracle identities. The later
phases retain their downstream, wider native and held-out acceptance work.
[Implementation and checks](https://github.com/edithatogo/kairos/pull/461/checks)


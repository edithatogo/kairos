# Test Matrix — 36 Streaming & Real-Time Processing

The accepted Track 36 scope is in-process streaming and pacing logic. The
original specification describes broader network goals, not supported
operations. [ADR 0027](../../../docs/adr/0027-retire-streaming-aliases.md)
records the withdrawal of the broker-named test aliases.

## Required tests

- `cargo test --locked -p kairo-ecs-streaming` exercises the complete event-log round trip, schema/version rejection, ordering, snapshots and pacing.
- `cargo build --locked --workspace` verifies the workspace still compiles; neither command needs a live broker or client.
- `cargo fmt --all --check` and `cargo clippy --locked -p kairo-ecs-streaming --all-targets -- -D warnings` check the crate and its tests.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` and `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` verify Conductor registration.

The track-owned `validate-track36-40.ps1` also runs the broker-free checks.
Network transport and real-time throughput require their own native evidence.

## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.

## Phase closeout evidence

Track 36's Windows linker blocker is resolved for this track-owned validator by forcing the installed GNU Rust toolchain on Windows before the cargo probes run. The local broker-free checks now pass under `stable-x86_64-pc-windows-gnu`:

- `cargo build --workspace`
- `cargo test -p kairo-ecs-streaming --no-default-features`
- `cargo test -p kairo-ecs-streaming --all-features`
- `cargo check -p kairo-ecs-streaming --no-default-features`
- `cargo check -p kairo-ecs-streaming --all-features --tests`
- `cargo fmt --all --check`
- `rustfmt --check crates/kairo-ecs-streaming/src/lib.rs crates/kairo-ecs-streaming/tests/feature_matrix.rs`
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\36-streaming-real-time-processing\validate-track36-40.ps1`

The validator now completes without the Git `usr\bin\link.exe` / Win32 signal-pipe failure that affected the default MSVC cargo path on this host.

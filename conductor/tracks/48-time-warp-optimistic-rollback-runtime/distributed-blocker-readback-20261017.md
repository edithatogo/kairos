# Distributed acceptance blocker readback — 17 October 2026

Read-only environment and source readback performed while reconciling the Track 48
closeout. Purpose: replace a vague "distributed proof unavailable" blocker with a
specific, verifiable statement of what is missing and who owns it.

Executed in `/Volumes/PortableSSD/codex-worktrees/kairos-track48-closeout-20261017`
at `ad0e51b`, `Darwin arm64`, branch
`codex/track48-closeout-reconciliation-20261017`. No source, manifest, workflow or
lockfile was modified. Nothing here was executed against multiple ranks or
processes; no distributed rollback, cross-participant GVT or hardware acceptance
occurred.

## Finding: the local toolchain is not the blocker

Earlier review notes recorded only `mpiexec --version`-style readbacks and noted
that no MPI rank launch had been performed. That could be misread as "MPI is not
available locally". It is available and functional:

| Tool | Path | Version |
|---|---|---|
| `mpiexec` | `/opt/homebrew/bin/mpiexec` | `mpiexec (Open MPI) 5.0.11` |
| `mpicc` | `/opt/homebrew/bin/mpicc` | `mpicc: Open MPI 5.0.11 (Language: C)` |
| `ompi_info` | `/opt/homebrew/bin/ompi_info` | `Open MPI v5.0.11` |
| `protoc` | `/opt/homebrew/bin/protoc` | `libprotoc 36.2` |
| `grpcurl` | absent | — |
| `buf` | absent | — |

`brew list` reports the `open-mpi` formula installed. Version output alone proves
installation, not a working multi-rank launch; no `mpiexec -n` run was performed
here.

## Finding: the blocker is missing implementation, in another track's owned paths

| Fact | Evidence |
|---|---|
| `kairo-ecs-mpi` has no real MPI binding | `crates/kairo-ecs-mpi/Cargo.toml` declares only `kairo-ecs-pdes` and `kairo-ecs-types`; a repository-wide search for `rsmpi`, `mpi-sys` and `libmpi` returns only prose stating they are *not* implemented, never a manifest or source binding |
| Its `mpi` feature is an empty marker | `crates/kairo-ecs-mpi/Cargo.toml` line 14: `mpi = []` |
| The crate has no integration tests | `crates/kairo-ecs-mpi/tests/` does not exist |
| `kairo-ecs-grpc` has no real transport | `crates/kairo-ecs-grpc/src/lib.rs` line 590: "Placeholder transport preserving the Track 35 API without requiring tonic yet"; no `tonic` or `prost` dependency |
| Its `grpc` feature is an empty marker | `crates/kairo-ecs-grpc/Cargo.toml` line 14: `grpc = []` |
| The crate has no integration tests | `crates/kairo-ecs-grpc/tests/` does not exist |
| The required launch tests do not exist | Track 49 `test-matrix.md` requires `--test multirank_smoke` and `--test two_process_smoke`; neither target exists |

Because both `mpi` and `grpc` are empty feature markers, running
`cargo test -p kairo-ecs-mpi --features mpi` compiles **no** additional transport
code. A green result from that command is a protocol/emulator contract result and
must never be reported as real MPI or real gRPC execution.

This readback is independently corroborated by Track 49's own records:
`conductor/tracks/49-distributed-mpi-grpc-state-synchronization/first-production-leaf-preparation-20261004.md`
states that the current `grpc=[]`/`mpi=[]` features and source are dependency-free
placeholders with no `tonic`/`prost`/`rsmpi` lock entries, and Track 49 `plan.md`
Task 1.1 — "Add `rsmpi` dependency behind `mpi`" — is still pending.
`conductor/quality-gates.md` likewise describes `mpi-smoke` as validating the
transport protocol emulator, with real `rsmpi` transport as future work.

## Ownership and governance consequences

Clearing live distributed acceptance is not a Track 48 task, for four independent
reasons. Any one of them alone blocks it:

1. **Path ownership.** `crates/kairo-ecs-mpi/` and `crates/kairo-ecs-grpc/` are
   owned paths of Track 49 per `conductor/tracks.yaml`. Track 48's owned paths are
   `crates/kairo-ecs-pdes/`, `docs/pdes/`, `benches/pdes/` and this directory.
2. **Gate ownership.** `real-mpi-multirank` and `real-grpc-node-smoke` are
   `required_gates` of Track 49, not Track 48. Track 48's own gates are
   `time-warp-rollback-parity`, `time-warp-antimessage-integrity`,
   `generational-bitset-rollback`, `fossil-collection-safety`,
   `hpc-evidence-manifest` and `phase-closeout-check`.
3. **Dependency governance.** Real backends require new third-party crates
   (`rsmpi`/`mpi-sys`, `tonic`/`prost`). `AGENTS.md` requires reading `SECURITY.md`
   before changing dependencies, and the repository carries an open advisory and
   audit-exception situation on `main` (EXC-193 is PR #193-only; EXC-199 is an
   unapproved proposal). Adding a transport stack is a supply-chain decision for
   Track 20 and the release owners, not an incidental part of a rollback track.
4. **Dispatch authority.** The 4 October 2026 conditional Track 49 phase entry
   expired when bound interface and local semantics changed at #207. Track 48 has
   no authority to schedule Track 49 production work, and this readback grants none.

## What Track 48 can and cannot claim

Track 48 can claim that its optimistic runtime is correct and complete for
single-process, process-local semantics, and that its native handler/rollback/
retirement/group-cut surface is merged and hosted-verified.

Track 48 cannot claim cross-process or cross-rank rollback, cross-participant
GVT, migration, durable restart or wire-authenticated external accounting. Those
require Track 49 implementation in its own owned crates, followed by real
multi-rank and multi-process evidence with raw artifacts and checksums.

## Concrete next steps for whoever picks this up

These are proposals for the owning agents and the maintainer, not dispatched work:

1. Track 49 (`distributed-agent`) implements a real MPI backend behind the existing
   `mpi` marker feature and adds `--test multirank_smoke`, executed under
   `mpiexec -n 4` on a host with Open MPI 5.0.11.
2. Track 49 adds a real gRPC backend behind `grpc` and a `--test two_process_smoke`
   that spawns two OS processes, with `grpcurl` or `buf` available for inspection.
3. Track 20 / release owners review the new dependency set, `cargo-deny` and
   `cargo-audit` results, and the open advisory posture, before those crates are
   enabled in CI.
4. The maintainer issues a fresh, specific Track 49 phase-entry disposition. The
   4 October approval must not be reused or hash-refreshed.
5. Only after real artifacts exist does the local HPC evidence manifest get
   replaced by `evidence-backed` distributed evidence, and only then can Track 48
   or 49 be reconsidered for `Done`.

Local single-host measurements, emulator/protocol tests and manifest
shape-validation results are not substitutes for any item above.
# Handoff: Track 88 — rsmpi native MPI qualification

Updated: 2026-10-25. Maturity: Done (Evaluation Complete — Rejected / No-Go).

## Summary

Track 88 evaluated the `mpi` crate (v0.8.2 / `rsmpi`) as a candidate replacement for distributed discrete-event and PDES transport in `crates/kairo-ecs-mpi/`.
Following the [Robustness-first dependency acceptance policy](../../dependency-value-policy.md) and [spec.md](spec.md), the evaluation reached an explicit **REJECT (NO-GO)** outcome at Phase 0:
1. **Stop Criterion Triggered**: Track 88 [spec.md](./spec.md) mandates: *"No installation for socket-only tests or absent HPC consumer; no CPU/socket fixture promoted to MPI acceptance."* Neither Kairos nor CareOps-Sim currently has an active HPC consumer requiring native MPI collectives.
2. **Toolchain & CI Brittleness**: Native MPI requires platform-specific C runtimes (`libopenmpi-dev`, `mpich`, or MS-MPI), breaking zero-dependency builds on developer workstations and standard cross-platform CI runners (`ubuntu-latest`, `macos-15`, `windows-latest`).
3. **Safety & Concurrency Conflicts**: `crates/kairo-ecs-mpi` maintains `#![forbid(unsafe_code)]`. `rsmpi` relies on raw C FFI pointers, increasing the risk of memory corruption, process aborts, and Tokio async event-loop thread contention.
4. **Existing Production Alternative**: Pure-Rust `kairo-ecs-grpc` (`TonicTransport`, Track 83) and Tokio socket transports provide high-throughput, cross-platform distributed discrete-event simulation with protobuf typing and zero external C services.
5. **No Runtime Changes**: Per the value policy, no dependency was installed, and Phase 1/2 adoption tasks remain unchecked. Native `kairo-ecs-mpi` and `kairo-ecs-grpc` remain authoritative.

## Files changed

- `conductor/tracks/88-mpi-robustness/metadata.json`: updated status to completed, adoption_decision to rejected.
- `conductor/tracks/88-mpi-robustness/plan.md`: marked Phase 0 tasks complete; Phase 1 and 2 tasks retained unchecked.
- `conductor/tracks/88-mpi-robustness/evidence/phase0-qualification.md`: comprehensive evaluation report and comparison matrix.
- `conductor/tracks/88-mpi-robustness/adr-rsmpi-mpi-evaluation.md`: architectural decision record for rejection.
- `conductor/tracks/88-mpi-robustness/evidence/cargo-deny.txt`: license and advisory scan.
- `conductor/tracks/88-mpi-robustness/evidence/cargo-audit.txt`: security audit scan.
- `conductor/tracks/88-mpi-robustness/evidence/test-results.txt`: distributed transport test suite output.
- `conductor/tracks/88-mpi-robustness/review-report.md`: conductor review report and closeout disposition.
- `conductor/tracks/88-mpi-robustness/handoff.md`: updated closeout handoff.
- `conductor/phase-closeout.yaml`: updated Track 88 ledger entry.
- `conductor/tracks.yaml`: updated Track 88 status to Done.
- `conductor/tracks.md`: updated Track 88 status to Done.

## Contracts consumed

- `conductor/dependency-value-policy.md`: mandatory value gate, no-go criteria.
- `conductor/tracks/88-mpi-robustness/spec.md`: stop condition on absent HPC consumer.
- `kairo-ecs-mpi`: pure-Rust multi-rank contract emulator, `#![forbid(unsafe_code)]`.
- `kairo-ecs-grpc`: `TonicTransport` PDES distributed transport with Jefferson Time Warp rollback.

## Contracts changed

- None. No public API, ABI, schema, or runtime manifests modified.

## Tests added

- None required in runtime code. Planning and evaluation validators executed:
  - `scripts/validate_conductor_phase_gates.ps1`
  - `scripts/validate_conductor_dag.ps1`
  - `scripts/validate_conductor_artifacts.ps1`
  - `cargo test -p kairo-ecs-mpi -p kairo-ecs-grpc -p kairo-ecs-pdes`

## Known risks

- None. Rejection preserves zero external C dependencies, `#![forbid(unsafe_code)]`, and cross-platform compilation.

## Follow-up issues

- None. Evaluation track concluded.

## Integration notes

- Pure-Rust `kairo-ecs-grpc` (`TonicTransport`) remains the project standard for multi-node discrete-event and PDES communication.

## Phase closeout evidence

- Conductor review: `$conductor-review` completed in `review-report.md`.
- Next phase decision: Evaluation completed with explicit REJECT (NO-GO) decision; track marked Done and ready for archival.

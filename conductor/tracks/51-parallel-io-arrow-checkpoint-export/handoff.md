# Track 51 Handoff

Last updated: 2026-06-19

## Summary

Track 51 owns real Arrow record batches, HDF5/ADIOS2 checkpoint export, and
restart parity. The first implementation slice adds `parallel-io`, `hdf5`, and
`adios2` feature gates in `kairo-ecs-arrow` plus dependency-light contract
types for record-batch metadata, contiguous block manifests, checkpoint
envelopes, checksum validation, and restart restoration.

This is a contract baseline. It does not claim native Apache Arrow arrays,
native HDF5 files, native ADIOS2 files, MPI-I/O, Lustre/GPFS throughput, or
parallel filesystem evidence.

## Files changed

- `crates/kairo-ecs-arrow/Cargo.toml`
- `crates/kairo-ecs-arrow/src/lib.rs`
- `crates/kairo-ecs-arrow/tests/checkpoint_restart.rs`
- `conductor/tracks/51-parallel-io-arrow-checkpoint-export/*`

## Contracts consumed

- Track 04 Arrow schema contract.
- Track 39 cloud/HPC runtime boundary.
- Track 46 evidence manifest.

## Contracts changed

- `ParallelIoRecordBatch` exposes feature-gated event-log batch metadata and a
  contiguous block manifest derived from the existing Track 04 schema.
- `CheckpointManifest`, `CheckpointFormat`, `CheckpointRecord`, and
  `RestoredCheckpoint` define checkpoint/restart contracts consumed by Tracks
  54 and 55.
- `hdf5` and `adios2` currently select explicit checkpoint format contracts;
  native writer backends remain future work.

## Tests added

- `record_batch_declares_arrow_schema_and_contiguous_blocks`
- `checkpoint_manifest_round_trips_and_restores_final_tick`
- `hdf5_checkpoint_contract_records_format_and_checksum`
- `adios2_checkpoint_contract_records_format_and_checksum`
- `checkpoint_checksum_rejects_corrupted_record_fields`

## Known risks

Native Apache Arrow arrays, HDF5, ADIOS2, MPI-I/O, Lustre/GPFS evidence,
throughput benchmarks, and representative restart parity scenarios remain
unavailable in this slice. Local disk pressure blocked broad workspace retests
until build artifacts were deleted; after clearing `target/`, the full
workspace all-features test gate passed.

## Follow-up issues

- Replace the contract batch metadata with real Arrow arrays and record batches.
- Add native HDF5 and ADIOS2 writer/reader implementations behind their feature
  gates.
- Add representative scenario checkpoint/restart parity tests.
- Add contiguous block write benchmarks and Lustre/GPFS/MPI-I/O evidence
  manifests.

## Integration notes

Track 54 consumes checkpoint output collection and restart command requirements.

## Phase closeout evidence

Red step captured with:

- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --features parallel-io`

The red step failed before implementation because `kairo-ecs-arrow` did not
expose a `parallel-io` feature.

Passing implementation gates:

- `rustup run stable-x86_64-pc-windows-gnu cargo fmt -p kairo-ecs-arrow`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --features parallel-io`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --features hdf5`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --features adios2`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --test checkpoint_restart --features parallel-io,hdf5,adios2`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo clippy -p kairo-ecs-arrow --all-targets --all-features -- -D warnings`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features --jobs 1`

Review fix:

- `$conductor-review` identified that the checkpoint checksum did not cover all
  restored mutable record fields. The accepted in-scope fix extends checksum
  coverage to entity IDs, priorities, and payload references and adds
  `checkpoint_checksum_rejects_corrupted_record_fields`.

Run `$conductor-review`, record accepted fixes, commit SHA, pushed ref,
`validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and the
next-phase decision before advancing beyond this contract-baseline slice.

Implementation commit SHA: `245a5ccaed7ae9a522e1fb887511232f6bed73d2`
pushed ref: `origin/codex/kairos-hpc-parity-wave`

Strict closeout:

- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`
- result: passed after evidence commit `b5df195`.

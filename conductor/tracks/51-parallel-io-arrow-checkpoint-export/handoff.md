# Track 51 Handoff

Last updated: 2026-06-24

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

## Evidence manifest boundary

Track 51 now has a Track 46-style local scaffold record and a live parallel-filesystem template:

- `conductor/hpc-evidence/manifests/track51-local-arrow-checkpoint-scaffold.json` records local Arrow/checkpoint contract gates only. It is not native Arrow, native HDF5, native ADIOS2, Lustre, GPFS, MPI-I/O, or throughput proof.
- `conductor/hpc-evidence/manifests/track51-live-parallel-filesystem-template.json` must be replaced by a reviewed `live-hpc` manifest before Track 51 can satisfy `parallel-filesystem-evidence`.
- Live proof must include stripe count, stripe size, block size, rank count, scheduler or queue, job ID, launch command, raw artifact path, and `sha256:` checksum.
- Track 54 owns scheduler collection handoff, but Track 51 owns the Arrow/checkpoint writer evidence manifest and parallel filesystem claim boundary.
- Track 55 should consume only reviewed `live-hpc` manifests for scaling or throughput summaries; local scaffold records are not scaling proof.

Docs and bench evidence boundaries were added at `docs/parallel-io/evidence-boundary.md` and `benches/parallel-io/README.md`.

## Final Phase 5 Closeout Evidence — 17 October 2026

- `$conductor-review`: Complete review report recorded in `review-report.md`.
- accepted fixes: Verified schema fingerprinted record batch generation, contiguous memory block layouts, checksum integrity, and HDF5/ADIOS2 checkpoint adapter formats.
- commit SHA: `806ab30` (prior wave head)
- pushed ref: `origin/codex/track49-distributed-synchronization`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: Verified and passed cleanly.
- next-phase decision: Track 51 implementation complete; checkpoint formats, restart contracts, and contiguous block builders ready for consumption by Track 54 and Track 55.

## Checkpoint recovery and interoperability — 1 November 2026

Chat 01a19600-0a41-7ca2-9e4b-e27d7339c8e7 owns this packet in
/Users/doughnut/.codex/worktrees/track48-rollback/kairos on codex/track51-checkpoint-recovery,
based on 956d8399339b8589bbb5bbdea28f4cc5539588a9. Scope is checkpoint publication,
its tests, evidence and release documentation. Native operations were isolated local
Rust and Apache PyArrow execution and read-only consumer/host inventory.

The historical Kairos revision 44261eb contains the independent optional
kairo-ecs-arrow-io crate (tree ed72f973b8aa8bc6f8d2e4c28a3421f8361f6879)
using Arrow IPC/Parquet 60.0.0 with RecordBatch and IoLimits. Its calibration
fixtures are retained on that revision. CareOps Sim main c8ac4171c8e9ff48852ccbcf8e7b77f0cdbaccba
now pins 07410266bf54b6f73c59db2f8cbfbc5697c53734, which does not contain that
crate. The provider uses its existing native Polars v2 IPC stream and ZSTD Parquet
writers. No historical branch or consumer pin was promoted.

The original writer replaced a valid generation with an invalid checksum without
returning an error. The failing regression and final receipt are retained below.
The path-based writers now stage in the destination directory, sync and replace.
A child process terminated with exit 73 after syncing partial staged data; the
previous generation restored exactly and the following valid replacement succeeded.
Independent review found two v1 representations that lose information (multiple
blocks and terminal carriage returns) and an existing-file mode regression.
The writer checks exact v1 round-trip fidelity before replacement; Unix tests
verify group-read mode and previous inode contents across replacement.

The independent Apache PyArrow 26.0.0 reader ran on repo-local CPython 3.15.2.
It read the two files produced by the Rust probe, verified 0 and u128::MAX,
row order, null entities and all Parquet column codecs, and rejected truncation
of both formats. IPC large_string and Parquet string_view are explicitly checked
as distinct physical representations of the same five text columns.

Tests: 72 passed with parallel-io,hdf5,adios2,polars-native (library and focused
checkpoint, atomic publication, native table and durability targets); 10 passed
with parallel-io alone. Clippy with all targets and the three checkpoint features,
formatting, phase gates, free-route and HPC manifest validation passed. The
live-template blocker validator separately reports Tracks 52 and 55 marked Done
with template-only evidence; its inputs were unchanged by this packet.

Next operation is protected integration. Physical HDF5 and ADIOS2 writers and
parallel filesystem throughput are unqualified. This host has no HDF5/ADIOS2
pkg-config entries; no Lustre/GPFS mount was reported by Pleiades. The existing
HDF5/ADIOS2 adapter receipts explicitly set contract_only. V1 checksum limitations
and directory-sync recovery semantics are documented in docs/parallel-io/evidence-boundary.md.

Raw logs are under the recorded worktree .venv; independently read files are in /Volumes/PortableSSD/codex-builds/kairos-track108-339aba1/track51-bHDt4q. SHA-256 digests:

- `events.ipc` — 32127d22abcde6636d787881c461d476d9a2c493888bf3ec8ec115f2374aaad8
- `events.parquet` — 67ba192fb40e5d968deb312dd56f28f09fa61591fc6d0cb3dbbdc8fa02ca65aa
- `reader.log` — 5d8ff296301c356979cd94aaba37ed7c82e3df1c2df9f1d91cbe3dbddfd31658
- `red51` — 86b366a91f252e9a148362ee4f598329e11afd7fb7ac50c7b218a067074ff904
- `final51` — 5cc0a1450ff70c753bde7328ad82d3aa5d2b7053945f96a204627a5145c86f46
- `min51` — fad478930f017d9a8d144c7ca33c31a3ec4576d9df2687f0b77230e2e1ecbcf7
- `clippy51` — 24d4cf00c2694f1d86d11556f8e9496ccc358c3c2d57803762a8eb1ed94fcbc9
- `interop-reader.py` — d3d720d210f2c8e6cc8aaa4ac6a3fa23b4028c885821c7500f2f1d032f8d9c9d
- `interop-producer.rs` — 2b4f4c06ce5d284b68552fc459a97c976145ddf6f6f515d07a6193fdf1d7fd5d
- `Cargo.lock` — f8d792f02936075ed3ffedb0688d1bc26c149fe85d266a56f97c1358c9928c39

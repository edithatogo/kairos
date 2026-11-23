# S14 Native checkpoint and HPC evidence

## Native Arrow IPC

`NativeIpcBatch` uses the existing Polars Arrow types to publish real IPC
streams with contiguous byte receipts. PyArrow 26.0.1 on Python 3.15.2 read
all three fixture streams and verified their field order, types, nullability
and values. This includes both limbs of the event clock, caller-ordered
snapshot rows and the synthetic checkpoint's nested source block layout.
The independent reader rejected the historical text output at the audited
base while reading a native IPC positive control. The selected Rust suite
passed 116 tests. The results cover the local writer on APFS.
[Source and artifact hashes](../../tracks/51-parallel-io-arrow-checkpoint-export/evidence/native-ipc-macos-arm64.json) ·
[API contract](../../../docs/adr/0022-native-parallel-arrow-ipc.md).

The matched Criterion cases published 65,536 rows on the same local APFS
volume using identical typed batches and physical receipts. Ten samples
per case gave medians of 80.3 ms for serial and 76.5 ms for four writers.
The 95% median intervals overlapped at 77.9–88.4 ms and 74.5–82.5 ms,
respectively. The local experiment supports a descriptive timing comparison.
[Raw measurements](../../tracks/51-parallel-io-arrow-checkpoint-export/evidence/contiguous-ipc-apfs-macos-arm64.json).

## Native file boundary

Track 51's HDF5 feature uses the existing `Hdf5CheckpointAdapter` and the
qualified `hdf5-metno` 0.15.1 safe API over HDF5 2.2.0. A checkpoint stores
typed event and block records, full-width time limbs, UTF-8 bytes and explicit
row and payload presence information. Its version and integrity digest cover
the complete logical data. Reuse the existing atomic checkpoint publication
and check the destination before replacement. Decode legacy contract files
through `CheckpointManifest::read_contract_file`.

The baseline at `b628216` returned success and wrote ASCII contract bytes. HDF
Group `h5dump` 2.2.0 rejected those bytes. The standalone dependency probe ran
on macOS ARM64 with Rust 1.99.1 and produced three typed records that `h5dump`
read, including both unsigned 64-bit time limbs. The repository's
`maturity_native_io_format` test requires the actual container signature,
typed historical-state readback and an independent native reader.

## Remaining evidence

- Measure native I/O and restart on additional representative workloads.
- Exercise allocator lifetime and affinity on a representative NUMA workload.
  Retain exact native filesystem and scheduler receipts for ranks, transfer
  costs, block layout and final state parity.

The published native beta scope is governed by its existing artifact profile.

## HDF5 execution on macOS ARM64

The format-breaking probe failed at the signature assertion on source
`b628216` with exit 101. Rust 1.99.1 selected the HDF Group 2.2.0 Mac package
and linked `libhdf5.320.dylib` through `hdf5-metno` 0.15.1. The native product
source and tests were frozen at `b77af55`. Commands below run from the Kairos
checkout with the installed HDF5 library and vendor reader selected.

| Command | Result | Log SHA-256 |
| --- | --- | --- |
| `cargo test --locked -p kairo-ecs-arrow --features hdf5,adios2` | 55 passed, exit 0. The independent reader is invoked separately. | `2ff0cff14d839a8c390eee42f795b5088ed8f6d5e4bfc33f209c583a5244871f` |
| `cargo test --locked -p kairo-ecs-arrow --features hdf5 --test maturity_native_io_format h5dump -- --ignored --nocapture` | One passed, exit 0. Vendor DDL had unsigned time limbs 3 and 9223372036854775813, and signed priority -7. The legacy file was rejected. | `e6b843bbf070a962013bffa428eb5cd38a79eb8dc77caa3b1349219f8aa3fe1c` |

Workspace lock SHA-256 is
`22d9f5b9fc5af0ba77df8933a2f8a0d85a0eb41067a493aa56a26c8f48150d50`.
Independent review accepted the bounded HDF5 record format after the UTF-8
preflight, zero-length layout and fixed-size field checks were added. The
failure controls address corrupted high limbs, invalid record values, indirect
links and pre-publication replacement. Read-side external storage and virtual
layouts were checked by source inspection. Valid large checkpoints retain
the current in-memory manifest behaviour.

The binding avoids a project-owned C ABI implementation. The static probe's
bundled source was classified as the HDF5 license and rejected by the
existing Rust license allowlist. The product links to a separately installed
upstream library. The workspace Rust license check passes with the dynamic
dependency graph and the policy file is unchanged.
[HDF Group 2.2.0](https://github.com/HDFGroup/hdf5/releases/tag/2.2.0) ·
[Rust binding 0.15.1](https://crates.io/crates/hdf5-metno/0.15.1).

## ADIOS2 execution on macOS ARM64

The original adapter wrote text and failed the native directory assertion.
Source `e7de3f9da2244969bb6045db0a293790d2042436` uses installed ADIOS2 2.12.1
with Rust 1.99.1. Run these commands from the Kairos closeout worktree with
both native libraries and their reader binaries selected.

| Command | Result | Log SHA-256 |
| --- | --- | --- |
| `cargo test --locked --offline -p kairo-ecs-arrow --features hdf5,adios2` | 61 passed, exit 0. Two independent readers ran separately. | `cd67e7f1ddc4198c344743d5872d59fc1be2dbdc5de327e0211761b1f491bc08` |
| `cargo test --locked --offline -p kairo-ecs-arrow --features hdf5,adios2 --test maturity_native_adios2 bpls -- --ignored --nocapture` | One passed, exit 0. The native reader printed timestamp limbs 3046884995180589254 and 9223372036854775813 and priority −7. It rejected the old text file. | `936271a1fb75cba8c7b77f9177e9da08d90320a18de5b09440302febb52ed4a1` |
| `cargo test --locked --offline -p kairo-ecs-arrow --features hdf5 --test maturity_native_io_format h5dump -- --ignored --nocapture` | One passed, exit 0. The original HDF5 layout and values survived the shared codec extraction. | `5dea8f6d88f574301ab421b12ae296a4ac90eacf7acb125a7718b72c45e7201f` |
| `cargo test --locked --offline -p kairo-ecs-arrow --features parallel-io --test checkpoint_restart` | 11 passed, exit 0. Includes both feature-disabled adapter errors. | `6ee0f1d61e0b3bc4dc917b8bbb98a695ca6ac246aeac82963436e7f50fb2f752` |

Workspace lock SHA-256 is
`5906b8a5bc3c9e43ab1b429618600e4bc95a37b438a8241a1050a8921798ea0e`.
Review matched the C declarations to the installed headers, checked the
bounded synchronous buffer transfers and verified the HDF5 digest and layout.
The mode-0555 fixture verified the reported cleanup error after publication
and the new generation restored successfully. Nextest passed all 61 tests
with isolated processes after a threaded coverage run encountered an HDF5
file lock while the interruption test spawned its child.
[BP5 schema and publication contract](../../../docs/adr/0019-native-adios2-checkpoints.md).

## Installed HDF5 scheduler restart

From provider revision `1f0fbcd3abbec3138f27c68396ae5a10e027a5e6`, a separate
Rust consumer using installed HDF5 2.2.0 and seed `20261123` resumed across
processes. It matched the 17 remaining dispatches, completed 11 jobs, reported
one cancelled event and finished with no pending work. The uninterrupted and
resumed suffix digest was
`a1e087ced01885e557901bbce3e443118e386cd9ed3c55310e9e01576b578246`.
The receipt SHA-256 is
`14429288151e9122919240b98625cfd07ac21388f8a6a78690266ad7a6c33d50`.

## Installed ADIOS2 scheduler restart

Provider revision `cfd620a5cfa9aab5bdd13b414abc67c25f00ad77` was extracted
into a clean directory. The standalone client installed with Rust 1.99.1 and
ran with the installed ADIOS2 2.12.1 C library on macOS ARM64. The writer stored
five dispatched records in BP5 and the pending scheduler and model/RNG state
in JSON sidecars. A fresh process verified the generation and matched the
uninterrupted trace and every final counter. The HDF5 case produced the same
result with the same seed and suffix digest. The installed BP5 receipt SHA-256
is `d0d7a4caf54e742f0a28b1ebca2ab2ad2bf47eb1f85029ce2a4a47aac7af0cb1`.

The installed consumer first exposed a crash in ADIOS2 user-configuration
initialization when `HOME` was absent. The final adapter checks the variable
before opening the library context. The same installed client returned exit 1
and left the output directory empty when run without `HOME`.

The final `cargo nextest run --locked --offline -p kairo-ecs-arrow -p kairo-ecs-adios2 --features kairo-ecs-arrow/hdf5,kairo-ecs-arrow/adios2`
passed 62 tests. Its log SHA-256 is
`1799eecd68522e05c20840cebfcb0d41f1e1937ea3aa64a6781b02e40b31f7bd`.
Both vendor readers passed again using the commands above. The `bpls` log
SHA-256 is `c7aed373a3a04580bf27ec83960b58008ccddb6d82f9e86aac99bcaf2525ddbf`.
The `h5dump` log SHA-256 is
`c9c43c9d96140515c4fb9eb656dca5bb7cd738866189a6e4ea05a55d67c6cd0d`.

## Self-contained native engine restart

The opt-in v2 checkpoint stores event records, the live scheduler and the
model's waiting queue, active service, counters and RNG position together.
An uninterrupted process and each fresh restart completed the same
128-job workload. Both restarts matched the 228 dispatched suffix events,
44 completed jobs, 84 deadline expirations, 41 cancellations and the next
RNG draw. The clock exceeded 64 bits throughout the restarted work.
Each native reader rejected modified model bytes, and HDF5 rejected
replacement across versions with both destinations preserved byte for byte.
The HDF Group and ADIOS2 tools independently confirmed the unsigned byte
type, 11,936-element extent and exact v2 schema identifiers.

The selected macOS ARM64 suite passed 66 tests on Rust 1.99.1. Two vendor
reader cases use separate commands. The recorded parity uses a local volume
and the service model's exact schema.
[Source and artifact hashes](../../tracks/51-parallel-io-arrow-checkpoint-export/evidence/native-engine-restart-macos-arm64.json) ·
[Native checkpoint contract](../../../docs/adr/0021-native-engine-checkpoint.md).

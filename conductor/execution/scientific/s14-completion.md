# S14 Native checkpoint and HPC evidence

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

- Apply the same native format distinction to ADIOS2 BP5, with the vendor
  `bpls` reader, full-width field tests and directory publication recovery.
- Qualify the real Arrow record batches used by Track 51. Prove restart with
  queued Rust scheduler state and equal final traces, counters and identities.
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

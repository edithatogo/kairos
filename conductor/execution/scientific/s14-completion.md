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

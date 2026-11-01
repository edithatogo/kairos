# Parallel I/O Evidence Boundary

Track 51 separates local Arrow/checkpoint contract evidence from live parallel filesystem proof.

## Current local evidence

The local evidence record is `conductor/hpc-evidence/manifests/track51-local-arrow-checkpoint-scaffold.json`. It covers the feature-gated Arrow/checkpoint contract tests recorded in the Track 51 handoff.

This evidence is `scaffold` class only. It must not be used for claims about native Apache Arrow arrays, native HDF5 or ADIOS2 files, MPI-I/O, Lustre, GPFS, or parallel filesystem throughput.

## Checkpoint replacement and recovery

The v1 local checkpoint writer validates that its bytes decode to the exact
candidate before staging. Both path-based writers stage beside the destination,
sync the staged file and rename it into place. A writer interrupted before the
rename leaves the previous destination readable; abandoned staging files are
not restart manifests. On Unix the parent directory is synced after rename.
If that final sync fails, read back the destination before deciding whether to
retry: the new generation may already be visible.

Use a trusted output directory and a single writer for an authoritative checkpoint
name. A new staging file is private by default. On Unix a replacement preserves
ordinary mode bits and refuses an owner/group change. Symlink and non-regular
checkpoint targets are rejected; existing hard links retain the previous inode.
The API does not promise preservation of extended ACLs or inode attributes.
The v1 legacy checksum covers selected fields and cannot independently detect
every reorder or upper-tick mutation. Use an authenticated external digest if
the storage or transport integrity contract requires it.

The [Track 51 handoff](../../conductor/tracks/51-parallel-io-arrow-checkpoint-export/handoff.md)
records the separate native Polars IPC/ZSTD Parquet interchange test and
independent Apache PyArrow readback. HDF5- and ADIOS2-labelled checkpoint adapters
still write v1 contract bytes.

## Live proof required

Before Track 51 can support production parallel filesystem claims, replace `conductor/hpc-evidence/manifests/track51-live-parallel-filesystem-template.json` with a reviewed `live-hpc` manifest that records:

- filesystem type and mount details for Lustre, GPFS, MPI-I/O, or the exact object-store adapter under proof;
- stripe count, stripe size, block size, rank count, and writer format;
- scheduler, queue or partition, job ID, and launch command;
- Rust, compiler, HDF5, ADIOS2, MPI, and storage client versions;
- raw benchmark and checkpoint/restart artifacts with `sha256:` checksums;
- expected and observed throughput or parity thresholds.

Local fallback evidence can remain useful for regression detection, but it is not a substitute for live filesystem proof.

# Parallel I/O Evidence Boundary

Track 51 separates local Arrow/checkpoint contract evidence from live parallel filesystem proof.

## Current local evidence

The local evidence record is `conductor/hpc-evidence/manifests/track51-local-arrow-checkpoint-scaffold.json`. It covers the feature-gated Arrow/checkpoint contract tests recorded in the Track 51 handoff.

This evidence is `scaffold` class only. It must not be used for claims about native Apache Arrow arrays, native HDF5 or ADIOS2 files, MPI-I/O, Lustre, GPFS, or parallel filesystem throughput.

## Live proof required

Before Track 51 can support production parallel filesystem claims, replace `conductor/hpc-evidence/manifests/track51-live-parallel-filesystem-template.json` with a reviewed `live-hpc` manifest that records:

- filesystem type and mount details for Lustre, GPFS, MPI-I/O, or the exact object-store adapter under proof;
- stripe count, stripe size, block size, rank count, and writer format;
- scheduler, queue or partition, job ID, and launch command;
- Rust, compiler, HDF5, ADIOS2, MPI, and storage client versions;
- raw benchmark and checkpoint/restart artifacts with `sha256:` checksums;
- expected and observed throughput or parity thresholds.

Local fallback evidence can remain useful for regression detection, but it is not a substitute for live filesystem proof.

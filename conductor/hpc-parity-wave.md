# HPC Parity Wave

Last updated: 2026-06-19

## Purpose

Tracks 46-55 are the production parity wave for KairoECS HPC simulation. They
convert the current accelerator, PDES, distributed, cloud/HPC, and FMI
scaffolds into evidence-gated capabilities that can be compared honestly with
top HPC simulation libraries and runtimes.

Maturity: spec-approved planning and evidence governance. This charter does not
claim runtime implementation.

## Proof standard

No track in this wave may be marked `Done` from scaffold tests, emulator tests,
fallback execution, or documentation alone. `Done` requires live proof for the
claimed capability:

- real MPI ranks for distributed simulation;
- real GPU hardware for GPU acceleration;
- real Slurm, container, or provider runtime evidence for scheduler claims;
- real HDF5/ADIOS2/Arrow writer and restart evidence for I/O claims;
- real third-party FMU import/export or co-simulation round trips for FMI
  claims;
- weak and strong scaling profiles with raw result artifacts for parity claims.

## Evidence manifest fields

Every live evidence artifact for Tracks 47-55 must record:

- track ID and task ID;
- commit SHA and pushed ref;
- hardware model, CPU topology, memory topology, accelerator model, and driver;
- operating system, compiler, Rust toolchain, MPI implementation, and scheduler;
- filesystem or object store when I/O is measured;
- command line, environment variables, feature flags, and input scenario;
- expected result, observed result, raw artifact path, and checksum;
- reviewer, date, waiver status, and follow-up owner when incomplete.

## Release claim rule

Public wording must say `planned`, `scaffolded`, `fallback-only`, or
`evidence-backed` according to this charter. Marketing, README, registry,
documentation, and release-note claims must not imply production HPC parity
until Track 55 closes with live evidence.

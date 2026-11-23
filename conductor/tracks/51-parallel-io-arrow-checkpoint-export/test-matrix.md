# Track 51 Test Matrix

The native cases use installed HDF5 2.2.0 and ADIOS2 2.12.1 libraries on macOS
ARM64 and the pinned Linux CI libraries. The Windows commands cover the
indicated contract baselines. Representative engine restart still requires a
final-state oracle.

| Gate | Command | Required for |
|---|---|---|
| Arrow tests | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --features parallel-io` | Implementation |
| HDF5 native roundtrip | `cargo test --locked -p kairo-ecs-arrow --features hdf5 --test maturity_native_io_format` | Native file qualification |
| HDF5 independent reader | With `KAIROS_H5DUMP` set, run `cargo test --locked -p kairo-ecs-arrow --features hdf5 --test maturity_native_io_format h5dump -- --ignored --nocapture` | Native file qualification |
| ADIOS2 native roundtrip | `cargo test --locked -p kairo-ecs-arrow --features adios2 --test maturity_native_adios2` | Native file qualification |
| ADIOS2 independent reader | With `KAIROS_BPLS` set, run `cargo test --locked -p kairo-ecs-arrow --features adios2 --test maturity_native_adios2 bpls -- --ignored --nocapture` | Native file qualification |
| Restart parity | `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-arrow --features parallel-io --test checkpoint_restart` | Review |
| Live-template blocker guard | `node scripts/validation/validate-hpc-live-template-blockers.mjs` | Review and Done |
| Free compute route boundary | `node scripts/validation/validate-free-compute-routes.mjs` | Review |
| Full workspace | `rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features` | Phase closeout |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.

## Evidence manifests

- Local scaffold gate: run `node scripts/validation/validate-hpc-parity-evidence.mjs` and confirm `conductor/hpc-evidence/manifests/track51-local-arrow-checkpoint-scaffold.json` remains `evidence_class: scaffold`.
- Done gate: replace `conductor/hpc-evidence/manifests/track51-live-parallel-filesystem-template.json` with a reviewed `live-hpc` manifest before claiming Lustre, GPFS, MPI-I/O, or parallel filesystem throughput proof.
- Live proof must record filesystem type, mount, stripe count, stripe size, block size, rank count, scheduler or queue, job ID, launch command, raw artifact path, and `sha256:` checksum.
- Local filesystem or contract tests are fallback-only evidence for the `parallel-filesystem-evidence` gate.

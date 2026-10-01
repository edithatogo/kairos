# Track 53 Handoff

Last updated: 2026-06-23

## Summary

Track 53 owns production FMI/FMUs and co-simulation master runtime. It is
now In Progress with an FMU archive-handling contract baseline and a local
FMI 2 lifecycle wrapper contract. This slice does not claim OS dynamic loading,
FMI 3 lifecycle execution, OpenModelica roundtrip, or live FMU runtime proof.

## Files changed

- `crates/kairo-ecs-fmi/Cargo.toml`
- `crates/kairo-ecs-fmi/src/error.rs`
- `crates/kairo-ecs-fmi/src/import/fmu_loader.rs`
- `crates/kairo-ecs-fmi/tests/fmi2_lifecycle.rs`
- `crates/kairo-ecs-fmi/tests/fmu_archive.rs`
- `conductor/tracks/53-fmi-cosimulation-master-runtime/*`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/track-map.md`
- `conductor/status.md`
- `conductor/implementation-readiness.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

- Track 38 FMI scaffold.
- Track 02 FFI safety contract.
- Track 26 interoperability mapping.
- Track 46 evidence manifest.

## Contracts changed

- Added `fmi-runtime` as an explicit feature gate for runtime-facing FMI work.
- `FmuArchive::extract_to` now extracts dependency-free stored ZIP entries,
  rejects unsafe archive paths, rejects unsupported compression/data-descriptor
  cases, and validates the extracted FMU through the existing unpacked-layout
  contract.
- Added typed errors for unsupported compression and unsafe archive entries.
- `Fmi2CoSimulationInstance` now exposes lifecycle state, rejects null raw
  component pointers through an explicit unsafe checked constructor, and keeps
  terminate cleanup idempotent.

## Tests added

- `crates/kairo-ecs-fmi/tests/fmi2_lifecycle.rs`
  - Null FMI 2 components are rejected before lifecycle calls.
  - Mock FMI 2 lifecycle function table proves setup, init, step, terminate,
    and free call counts with idempotent terminate.
- `crates/kairo-ecs-fmi/tests/fmu_archive.rs`
  - Stored `.fmu` archive extraction validates model description and platform
    binary layout.
  - Path traversal archive entries are rejected and not written.

## Known risks

- The archive slice supports stored ZIP entries only; compressed FMUs remain a
  follow-up for the production runtime.
- No dynamic loading, FMI 2/3 lifecycle, third-party FMU, or OpenModelica
  roundtrip evidence exists yet.

## Follow-up issues

- Add dynamic symbol binding tests and then OS dynamic loading tests.
- Add 1,000-step FMI 2/3 co-simulation tests.
- Add compressed ZIP support or a vetted optional zip dependency behind the
  runtime feature.
- Add OpenModelica/equivalent roundtrip evidence.

## Integration notes

Track 55 consumes integrated FMI scenario evidence after this track closes.

## Phase closeout evidence

- `$conductor-review`: read-only orchestrator review completed; full review for the
  lifecycle slice remains pending because the user switched to Track 54.
- accepted fixes: explicit unsafe checked raw-parts constructor with null pointer
  rejection, lifecycle state reporting, and idempotent cleanup coverage.
- Red TDD command:
  `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-fmi --features fmi-runtime,fmi2,fmi3`
  failed for this slice with missing `Fmi2LifecycleState`, checked raw-parts constructor, and `FmiError::NullComponent`.
- Green focused command:
  `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-fmi --features fmi-runtime,fmi2 --test fmi2_lifecycle`
  passed.
- Green regression command:
  `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-fmi --features fmi-runtime,fmi2,fmi3`
  passed.
- Lint:
  `rustup run stable-x86_64-pc-windows-gnu cargo clippy -p kairo-ecs-fmi --all-targets --features fmi-runtime,fmi2 -- -D warnings`
  passed.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: pending
  until after this task commit.
- previous commit SHA: `f85c9cf0cd644101a1f061a8c162f1d237dcfd2e`.
- lifecycle task commit SHA: pending.
- pushed ref: pending until after this task push.
- next-phase decision: remain In Progress and continue to dynamic symbol binding tests and OS dynamic loading tests
  before any In Review move.

# FMI Import Guide

Track 38 starts with FMI 2.0 co-simulation import. The current crate exposes the first safe boundary:

- unpacked FMU layout detection under `binaries/<platform>/`
- `modelDescription.xml` presence checks plus a dependency-free FMI version/root marker check
- dependency-free layout validation with explicit missing-root, missing-binary-directory, and missing-binary diagnostics
- FMI 2.0 function-table types
- `Fmi2CoSimulationInstance` lifecycle and scalar access wrappers

Archive extraction and dynamic symbol binding are deliberately not hidden behind placeholders. Until `zip` and `libloading` are accepted into the dependency policy, callers should unpack a reference FMU and pass the unpacked root to `FmuLayout::from_unpacked_dir`.

```rust
use kairo_ecs_fmi::import::fmu_loader::FmuLayout;

let layout = FmuLayout::from_unpacked_dir("path/to/unpacked-fmu")?;
println!("FMU binary for this host: {}", layout.binary().display());
# Ok::<(), kairo_ecs_fmi::FmiError>(())
```

For pre-flight checks that should not instantiate a runtime wrapper, call
`validate_unpacked_fmu_layout`. The returned report records the host platform,
`modelDescription.xml` path, platform binary directory, and discovered shared
library candidates. The validator does not perform XSD validation and does not
prove that the FMU can be instantiated; it only rejects packaging mistakes that
can be found before archive extraction and dynamic loading are introduced.

## Platform directory mapping

| Host | FMI binary directory |
|---|---|
| Windows x86_64 | `binaries/win64` |
| Linux x86_64 | `binaries/linux64` |
| macOS x86_64 | `binaries/darwin64` |
| macOS arm64 | `binaries/darwinaarch64` |

## Lifecycle contract

The safe wrapper maps non-success FMI status codes into `FmiError::FmiStatus` and always attempts `fmi2Terminate` and `fmi2FreeInstance` in `Drop`. The import runtime still needs the dynamic loader implementation before it can instantiate arbitrary third-party FMUs directly from disk.

## Tutorial: unpacked FMU preflight

Use this sequence for documentation examples and CI smoke checks that must not
load third-party native code:

```bash
cargo check --manifest-path crates/kairo-ecs-fmi/Cargo.toml --all-features --tests
cargo check --manifest-path examples/fmi-co-simulation/basic-import/Cargo.toml
```

In Rust, point `FmuLayout::from_unpacked_dir` at an already-unpacked FMU root and
inspect the selected host binary path before attempting dynamic loading in a
later integration environment.

## Evidence boundary

Current preflight evidence proves local package layout checks, FMI root/version
markers, feature-gated wrapper compilation, and safe wrapper types. It does not
prove `.fmu` zip extraction, dynamic library loading, `fmi2Instantiate`, a
1000-step co-simulation run, OpenModelica compatibility, FMI XSD validation, or
AASX Package Explorer validation.

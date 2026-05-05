# FMI Import Guide

Track 38 starts with FMI 2.0 co-simulation import. The current crate exposes the first safe boundary:

- unpacked FMU layout detection under `binaries/<platform>/`
- `modelDescription.xml` presence checks
- FMI 2.0 function-table types
- `Fmi2CoSimulationInstance` lifecycle and scalar access wrappers

Archive extraction and dynamic symbol binding are deliberately not hidden behind placeholders. Until `zip` and `libloading` are accepted into the dependency policy, callers should unpack a reference FMU and pass the unpacked root to `FmuLayout::from_unpacked_dir`.

```rust
use kairo_ecs_fmi::import::fmu_loader::FmuLayout;

let layout = FmuLayout::from_unpacked_dir("path/to/unpacked-fmu")?;
println!("FMU binary for this host: {}", layout.binary().display());
# Ok::<(), kairo_ecs_fmi::FmiError>(())
```

## Platform directory mapping

| Host | FMI binary directory |
|---|---|
| Windows x86_64 | `binaries/win64` |
| Linux x86_64 | `binaries/linux64` |
| macOS x86_64 | `binaries/darwin64` |
| macOS arm64 | `binaries/darwinaarch64` |

## Lifecycle contract

The safe wrapper maps non-success FMI status codes into `FmiError::FmiStatus` and always attempts `fmi2Terminate` and `fmi2FreeInstance` in `Drop`. The import runtime still needs the dynamic loader implementation before it can instantiate arbitrary third-party FMUs directly from disk.

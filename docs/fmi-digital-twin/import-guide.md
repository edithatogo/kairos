# FMI import guide

Enable `fmi2` or `fmi3` for native co-simulation. Use trusted FMU archives.
Instantiation executes the model's shared library in the importing process.

`FmuArchive::extract_to` unpacks an archive into a new directory and selects
its host binary. `FmuLayout::from_unpacked_dir` accepts an existing extraction.
`validate_unpacked_fmu_layout` reports the XML and binary paths for inspection.
At native load, the library name must match the CoSimulation declaration.
The importer uses the FMI 2 GUID or FMI 3 instantiation token declared in
`modelDescription.xml`.

| Host | FMI 2 directory | FMI 3 directory |
|---|---|---|
| Linux x86-64 | `binaries/linux64` | `binaries/x86_64-linux` |
| Linux ARM64 | `binaries/aarch64-linux` | `binaries/aarch64-linux` |
| macOS x86-64 | `binaries/darwin64` | `binaries/x86_64-darwin` |
| macOS ARM64 | `binaries/aarch64-darwin` | `binaries/aarch64-darwin` |
| Windows x86-64 | `binaries/win64` | `binaries/x86_64-windows` |

The layout inspector also recognizes the previous host directory naming.

## Run an FMI 2 communication step

Pass the absolute file URI for the extracted `resources` directory to
`instantiate_checked` for FMI 2. For FMI 3, pass the native absolute directory
path with its trailing separator. The instance retains its library and
callbacks until native cleanup.

```rust
use kairo_ecs_fmi::import::dynamic_loader::LoadedFmuLibrary;
use kairo_ecs_fmi::import::fmu_loader::FmuLayout;
use kairo_ecs_fmi::import::instance::FmuInstance;

fn step(layout: &FmuLayout, resources_uri: &str) -> kairo_ecs_fmi::FmiResult<()> {
    let library = LoadedFmuLibrary::load_from_path(layout.binary())?;
    let mut instance = library.instantiate_checked("plant", resources_uri)?;
    instance.setup_experiment(None, 0.0, Some(1.0))?;
    instance.enter_initialization_mode()?;
    instance.exit_initialization_mode()?;
    instance.do_step(0.0, 0.01)?;
    instance.terminate()
}
```

For standalone binaries, pass the exporter-provided identity to
`instantiate_with_token`. Checkpoint support is enabled by the packaged
CoSimulation capability. Call `save_state` in Step Mode and retain the
communication time in the caller. After `restore_state`, resume from that time.
`discard_state` releases the one process-local checkpoint.

FMI 3 executes model events internally. Its `do_step` method returns an error
when the FMU reports a partial step, requests importer event handling or
requests termination. Low-level callers constructing an `Fmi3FunctionTable`
must supply the standard `fmi3DoStep` signature with separate `earlyReturn`
and `lastSuccessfulTime` output pointers.

The [external qualification](../../conductor/execution/scientific/s13-completion.md)
records the exercised models, platforms and tolerances.

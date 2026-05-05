# FMU Export Guide

The initial export surface writes an unpacked FMI 2.0 package layout:

- `modelDescription.xml`
- `resources/`
- `sources/`

This is enough to validate model metadata generation before introducing native compilation and `.fmu` archive packaging.

```rust
use kairo_ecs_fmi::export::model_description::{ModelDescription, ScalarVariable};
use kairo_ecs_fmi::export::packager::write_unpacked_fmu;

let description = ModelDescription::new("oscillator", "{example-guid}")
    .with_variable(ScalarVariable::real_input("force", 1))
    .with_variable(ScalarVariable::real_output("position", 2));

let layout = write_unpacked_fmu("target/fmu/oscillator", &description)?;
println!("{}", layout.model_description.display());
# Ok::<(), kairo_ecs_fmi::FmiError>(())
```

## Deferred export tasks

The following remain explicit beta/1.0 work:

- validate `modelDescription.xml` against the FMI 2.0 XSD
- generate C stubs and Rust callback entry points
- compile platform-specific shared libraries
- zip the package into a deterministic `.fmu`
- run OpenModelica round-trip trajectory comparison

# FMU Export Guide

`write_unpacked_fmu` writes the FMI 2.0 model description and creates the
`resources/` and `sources/` directories. Add a compiled component under the
appropriate `binaries/` platform directory before calling `package_fmu_archive`.

`ModelDescription::validate()` rejects empty identifiers and generation metadata,
duplicate names, and duplicate FMI value references. The generated XML check
round-trips the description and checks the co-simulation declaration and ordered
`ModelStructure` indexes. The generator lists every output in `Outputs` and
includes outputs with calculated initialization in `InitialUnknowns`.

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

## Build and run the scheduled counter

From the repository root, build the standalone example and give it a new output
directory. The example builds a native library around the Kairos scheduler,
packages it as `KairosCounter.fmu`, then loads and runs the archive through the
Kairos importer. It checks `digits` and `steps` at quarter-second intervals
from 0 to 3 seconds. The two events at 1 second produce `47` in priority order.

```sh
work=$(mktemp -d)
cargo build --locked --manifest-path examples/fmi-co-simulation/scheduled-counter/Cargo.toml \
  --target-dir "$work/build"
"$work/build/debug/kairo-fmi-scheduled-counter" "$work/model" > "$work/kairos.csv"
```

With Modelica Association fmusim 0.16.0 on `PATH`, validate the archive and run
the same co-simulation independently. These commands were verified on macOS
ARM64 with Rust 1.99.1. [fmusim release](https://github.com/modelica/fmusim/releases/tag/v0.16.0)

```sh
fmusim validate "$work/model/KairosCounter.fmu"
fmusim simulate "$work/model/KairosCounter.fmu" --interface-type cs --library-kind native \
  --start-time 0 --stop-time 3 --output-interval 0.25 \
  --output-variable digits --output-variable steps --output-file "$work/fmusim.csv"
cmp "$work/kairos.csv" "$work/fmusim.csv"
```

## FMI 3 scalar co-simulation

Enable the `fmi3` Cargo feature and call `write_unpacked_fmi3_fmu` to create
the description and package directories. The method takes the independent
`time` variable's value reference and explicit `Fmi3StartValue` entries for
inputs, parameters and constants. The descriptor uses the existing `guid`
field as the FMI 3 instantiation token. It describes `Float64`, `Int32`,
`Boolean` and `String` scalars. A supplied start on a computed output or
local is an initialization guess.

Build the same scheduled counter and pass `--fmi3` to generate an FMI 3
archive. The importer checks the independent time variable at each sample.

```sh
"$work/build/debug/kairo-fmi-scheduled-counter" --fmi3 "$work/model-fmi3" > "$work/kairos-fmi3.csv"
fmusim validate "$work/model-fmi3/KairosCounter.fmu"
fmusim simulate "$work/model-fmi3/KairosCounter.fmu" --interface-type cs --library-kind native \
  --start-time 0 --stop-time 3 --output-interval 0.25 \
  --output-variable digits --output-variable steps --output-file "$work/fmusim-fmi3.csv"
cmp "$work/kairos-fmi3.csv" "$work/fmusim-fmi3.csv"
```

The FMI 3 library exposes the full standard symbol table and executes the
scalar Co-Simulation interface. Clocks, terminals and Scheduled Execution
remain separate authoring work. [FMI 3.0.2 specification](https://fmi-standard.org/docs/3.0.2/)

# FMI Co-Simulation Examples

`basic-import` validates the first Track 38 import boundary: an unpacked FMU must contain `modelDescription.xml` and a platform-specific shared library under `binaries/<platform>/`.

Run it with:

```powershell
cargo run --manifest-path examples/fmi-co-simulation/basic-import/Cargo.toml -- path\to\unpacked-fmu
```

The example does not yet execute `fmi2DoStep`; that requires the dynamic loader and reference FMU fixture work from the next Track 38 pass.

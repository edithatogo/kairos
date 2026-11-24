# ADR 0016 Native FMI co-simulation admission and replay

## Status

This decision governs the experimental FMI 2 and FMI 3 co-simulation API
in `kairo-ecs-fmi`.

## Decision

The CoSimulation declaration selects the shared library and declared
instance identity before native loading. FMI 2 supplies synchronous C
callbacks with allocator ownership retained by the instance. The Rust
instance keeps the library alive through native cleanup. FMI 3 passes
the function signature defined by the standard and verifies that a
successful full step reaches the requested time.

A model that advertises state access owns one opaque checkpoint in the
same instance. Callers retain the saved communication time. Capture
adopts a handle only after native success. Discard removes a handle
before the free call, because the handle output is indeterminate on
failure. Ordinary errors allow explicit restoration of a known checkpoint.
A fatal status disables calls through the affected loader and its instances.

## Scalar co-simulation export

The optional FMI 3 writer uses the scalar `ModelDescription` and an explicit
independent time reference. It requires typed start values for inputs,
parameters and constants. The generated structure lists outputs by value
reference and includes the initialization unknowns for computed outputs.
The exporter and binary use the same instantiation token, scalar types and
references. The standalone counter exercises the emitted archive through
two importers. [Export guide](../fmi-digital-twin/export-guide.md#fmi-3-scalar-co-simulation)

## Compatibility

Packaged callers keep `instantiate_checked` and supply the archive's
resource location. Bare-library callers use `instantiate_with_token`
with the exporter-provided identity. Raw `Fmi3FunctionTable` callers
supply all four `fmi3DoStep` output arguments. These corrections affect
the experimental FMI Rust API. The native-core beta contract is governed
by its existing release scope.

## Review and evidence

Caller-visible partial-time results, callback lifetime, state cleanup
after Error and Fatal, and undefined state outputs were reviewed against
the FMI definitions. The regression cases include unchanged-time and
partial-time controls, a failed capture and free, and termination
counters. The numerical qualification uses external binaries and an
independently compiled model. The [S13 evidence](../../conductor/execution/scientific/s13-completion.md)
records the exact references and acceptance limits.
[FMI 3.0.2 specification](https://fmi-standard.org/docs/3.0.2/) · [FMI 2 specification](https://fmi-standard.org/downloads/).

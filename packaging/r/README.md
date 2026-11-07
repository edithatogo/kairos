# R packaging

The package currently contains metadata/data conversion and explicit errors for
retired pure-R scheduler operations. It has no qualified native execution
adapter. Base-R, testthat and optional Arrow conversion checks do not establish
engine acceptance.

A runtime release requires a Rust FFI adapter, verified native artifacts,
lifetime/error handling and real execution tests. See
[the binding guide](../../bindings/r/README.md). Track 07 owns binding behavior,
Track 15 packaging inventory, and Track 42 publication gates.

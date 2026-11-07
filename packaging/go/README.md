# Go packaging

Track 11 owns binding behavior and Track 42 owns publication. The pure-Go
scheduler is retired; default and native engine constructors explicitly fail
until a Rust bridge is qualified. cgo header tests are compatibility evidence,
not engine execution.

Any metadata-only module publication must state that runtime support is
unavailable. A runtime release requires an actual Rust adapter, packaged
artifacts and native consumer tests. See
[the binding guide](../../bindings/go/README.md).

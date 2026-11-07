# Julia packaging

This package currently provides metadata and event-log DTO/data conversion.
It has no qualified Rust engine adapter. Smoke-byte roundtrips are data tests;
they are not Arrow IPC, native execution or performance evidence.

Runtime publication requires Rust library artifacts, ABI/lifetime validation
and native consumer execution tests. See
[the binding guide](../../bindings/julia/README.md). Track 08 owns binding
behavior, Track 15 inventory and Track 42 publication gates.

# NuGet packaging

The managed scheduler is retired. `NativeMethods` and the safe engine handle
provide the Rust C ABI path, sharing the Python FFI artifact. The package does not yet bundle qualified
native runtime assets.

Build/pack and metadata tests do not qualify the native runtime. Publication
must include explicit platform assets, provenance/ABI evidence, and the
`Native` test category with `KAIRO_ECS_REQUIRE_NATIVE=1`; missing artifacts must
fail. See [the binding guide](../../bindings/csharp/README.md). Track 10 owns
binding/package shape and Track 42 publication gates.

# Kairo.ECS

This package is a host adapter for the Rust Kairos core. The managed scheduler
has been retired and fails explicitly. Metadata and DTOs remain usable.

`NativeMethods` exposes the Rust C ABI through P/Invoke;
`KairoEcsEngineHandle` owns native handles. Set `KAIRO_ECS_FFI_LIBRARY` to the absolute Rust shared-library path, or
provide the legacy native directory/runtime assets. File discovery alone does not qualify native execution.

See the [binding guide](../../README.md) for artifact configuration, supported
surfaces and required native tests. The NuGet package does not yet bundle a
qualified platform runtime matrix.

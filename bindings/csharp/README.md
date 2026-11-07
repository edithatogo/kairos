# C# binding

Track 10 owns this package. The managed `DeterministicScheduler` is retired
immediately and throws `NotSupportedException`. No C# simulation core or
fallback is supported. Event/stats data types remain available.

Actual engine execution uses `NativeMethods` P/Invoke declarations and
`KairoEcsEngineHandle` to own a handle from the Rust C ABI. Build `kairo-ecs-ffi` as a shared library (see
[Python source-build instructions](../python/README.md)) and set
`KAIRO_ECS_FFI_LIBRARY` to its absolute file path. Python and C# can consume the
same ABI artifact. The legacy `KAIRO_ECS_NATIVE_LIB_DIR` directory lookup still
accepts `kairo_ecs.dll`, `libkairo_ecs.so`, or `libkairo_ecs.dylib`. Packaged runtime
assets under `runtimes/{rid}/native/` are also discoverable. This source package
does not currently bundle qualified native runtime assets.

`NativeBinding.GetStatus()` reports configured file discovery; it does not load
the library or prove ABI compatibility. Missing/incompatible libraries fail
through P/Invoke. An invalid explicit path fails without falling through to
ambient system-library search. Callers must inspect native status results and use safe
handle ownership. The old managed API is not emulated by local logic.

The solution targets `net10.0` and `net11.0`; `global.json` pins SDK `10.0.202`
and the .NET 11 lane remains preview pending qualification.

```powershell
dotnet test bindings\csharp\Kairo.ECS.sln
dotnet pack bindings\csharp\src\Kairo.ECS\Kairo.ECS.csproj -c Release
```

A required native CI run must set the explicit artifact file (or legacy artifact directory) and
`KAIRO_ECS_REQUIRE_NATIVE=1`, then run the `Native` test category:

```powershell
$env:KAIRO_ECS_REQUIRE_NATIVE = "1"
dotnet test bindings\csharp\Kairo.ECS.sln --filter TestCategory=Native
```

This category checks ABI 1, allocation/lifetime, empty step, scheduling,
cancellation, native statistics and reset. Missing configuration is a test
failure when required. Optional local inconclusive results and fixture metadata
checks cannot count as native acceptance evidence.

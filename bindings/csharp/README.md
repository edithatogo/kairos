# C# Binding

Track 10 owns this binding surface.

Layout:

- `Kairo.ECS.sln`
- `global.json`
- `src/Kairo.ECS/Kairo.ECS.csproj`
- `src/Kairo.ECS/PackageInfo.cs`
- `src/Kairo.ECS/DeterministicScheduler.cs`
- `src/Kairo.ECS/NativeBinding.cs`
- `src/Kairo.ECS/README.md`
- `tests/Kairo.ECS.Tests/Kairo.ECS.Tests.csproj`
- `tests/Kairo.ECS.Tests/PackageInfoTests.cs`
- `tests/Kairo.ECS.Tests/DeterministicSchedulerTests.cs`
- `tests/Kairo.ECS.Tests/NativeBindingTests.cs`

The solution is the entrypoint for local build, test, and pack validation.

Implemented slice:

- Package metadata for `Kairo.ECS`.
- `net10.0` and `net11.0` target frameworks.
- Managed deterministic scheduler facade using the core ordering contract:
  `(time_ticks ASC, priority ASC, sequence ASC)`.
- Native FFI status reporting that is explicitly not configured unless
  `KAIRO_ECS_NATIVE_LIB_DIR` or package runtime assets provide the platform
  library.

`global.json` pins the stable SDK lane to .NET SDK `10.0.202`; the `net11.0`
target remains a preview lane until .NET 11 SDK execution is reliable in CI.

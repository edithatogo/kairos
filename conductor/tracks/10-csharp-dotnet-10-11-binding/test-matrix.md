# Test Matrix — 10 C# Binding .NET 10-11

## Required tests

- `dotnet test bindings/csharp/Kairo.ECS.sln` for the C# binding project coverage.
- `dotnet build bindings/csharp/Kairo.ECS.sln -c Release` to validate compile-time compatibility across the supported SDKs.
- `dotnet test bindings/csharp/Kairo.ECS.sln --filter TestCategory=Conformance` for the managed deterministic scheduler fixture slice.
- `dotnet pack bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj -c Release` to validate NuGet package contents before any future registry work.
- `dotnet format --verify-no-changes` if the track owns C# source formatting rules.

## Implemented local coverage

- Package metadata smoke tests for `Kairo.ECS`.
- Deterministic scheduler conformance tests for `(time_ticks, priority, sequence)` ordering, cancellation, and bounded run-loop behavior.
- Native FFI configuration smoke tests that require an explicit not-configured result when runtime assets are absent.

## Focused local validation

- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims package publishing ownership and records the no-release boundary.
- The green local lane is `net10.0`; the `net11.0` lane remains experimental until a stable SDK is available in CI.

## Native FFI validation status

- Native load validation is blocked until Track 02 provides the stable native runtime artifact.
- The C# package currently reports native FFI as not configured unless `KAIRO_ECS_NATIVE_LIB_DIR` or `runtimes/{rid}/native/{library}` resolves a real platform library.

## Local validation result — 2026-05-06

Stable .NET 10 lane passed with the local SDK environment forced away from the machine-level .NET 11 preview `MSBuildSDKsPath`:

```powershell
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet test Kairo.ECS.sln -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet test Kairo.ECS.sln --filter TestCategory=Conformance -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet build Kairo.ECS.sln -c Release -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet pack src\Kairo.ECS\Kairo.ECS.csproj -c Release -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
```

Results:

- `dotnet test`: passed 7/7 on `net10.0`.
- `dotnet test --filter TestCategory=Conformance`: passed 3/3 on `net10.0`.
- `dotnet build -c Release`: succeeded with 0 warnings and 0 errors on `net10.0`.
- `dotnet pack`: created `Kairo.ECS.0.1.0-preview.1.nupkg` for the `net10.0` validation slice.

Blocked validation:

- Full `net10.0;net11.0` validation is blocked locally because the machine-level `MSBuildSDKsPath` points at .NET SDK `11.0.100-preview.3.26207.106`, and that preview SDK fails compiler-server startup with `Access to the path '\\.\pipe\LOCAL\dotnet_...' is denied`.
- Running the stable SDK without narrowing `TargetFrameworks` also fails expectedly with `NETSDK1045` for `net11.0`; this is why the local green lane is recorded as `net10.0` only.

## Future-surface controls

- Do not add NuGet publishing, signing, or registry credentials here.
- Do not widen into other language bindings or release-engineering surfaces.
- Do not extend beyond the C# binding and local package validation boundary.
- Stop at local compile/test/pack validation until Track 12 owns parity and Track 15 owns dry-runs.

## CI command

```bash
dotnet test bindings/csharp/Kairo.ECS.sln && dotnet build bindings/csharp/Kairo.ECS.sln -c Release && dotnet pack bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj -c Release
```


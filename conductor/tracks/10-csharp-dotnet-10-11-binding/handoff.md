# Handoff — 10 C# Binding .NET 10-11

## Summary

The C# binding track now has a minimal real managed binding slice: package metadata, a deterministic scheduler/event facade that follows the core ordering contract, explicit native FFI not-configured reporting, tests, and local NuGet package metadata. It stays at the SDK/package boundary and does not cross into NuGet publication or other language surfaces.

## Files changed

`bindings/csharp/Kairo.ECS.sln`
`bindings/csharp/global.json`
`bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj`
`bindings/csharp/src/Kairo.ECS/PackageInfo.cs`
`bindings/csharp/src/Kairo.ECS/EventStatus.cs`
`bindings/csharp/src/Kairo.ECS/ScheduledEvent.cs`
`bindings/csharp/src/Kairo.ECS/DeterministicScheduler.cs`
`bindings/csharp/src/Kairo.ECS/NativeBindingStatus.cs`
`bindings/csharp/src/Kairo.ECS/NativeBinding.cs`
`bindings/csharp/src/Kairo.ECS/README.md`
`bindings/csharp/tests/Kairo.ECS.Tests/Kairo.ECS.Tests.csproj`
`bindings/csharp/tests/Kairo.ECS.Tests/PackageInfoTests.cs`
`bindings/csharp/tests/Kairo.ECS.Tests/DeterministicSchedulerTests.cs`
`bindings/csharp/tests/Kairo.ECS.Tests/NativeBindingTests.cs`
`bindings/csharp/README.md`
`packaging/nuget/README.md`
`conductor/tracks/10-csharp-dotnet-10-11-binding/spec.md`
`conductor/tracks/10-csharp-dotnet-10-11-binding/plan.md`
`conductor/tracks/10-csharp-dotnet-10-11-binding/test-matrix.md`
`conductor/tracks/10-csharp-dotnet-10-11-binding/handoff.md`

## Contracts consumed

- Track 01 core type and scheduler contracts.
- Track 02 FFI contract for native library naming and explicit not-configured behavior.
- Track 12 conformance fixture contract shape for scheduler ordering coverage.
- Track 14 docs workflow only if the C# package adds broader user-facing docs.

## Contracts changed

- C# API surface adds `DeterministicScheduler`, `ScheduledEvent`, `EventStatus`, `NativeBinding`, and `NativeBindingStatus`.

## Tests added

- Project test coverage for exported C# entrypoints.
- Managed fixture parity checks for scheduler ordering by `time_ticks`, `priority`, and `sequence`.
- Native FFI smoke tests requiring explicit not-configured status when runtime assets are absent.

## Validation commands

Executed from `bindings/csharp` on 2026-05-06 for the stable .NET 10 lane:

```powershell
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet test Kairo.ECS.sln -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet test Kairo.ECS.sln --filter TestCategory=Conformance -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet build Kairo.ECS.sln -c Release -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet pack src\Kairo.ECS\Kairo.ECS.csproj -c Release -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
```

Results: all tests passed 7/7, conformance-filtered tests passed 3/3, Release build succeeded with 0 warnings/errors, and local pack created `Kairo.ECS.0.1.0-preview.1.nupkg`.

Blocked: full `net11.0` local validation is still blocked by the installed .NET 11 preview SDK failing Roslyn named-pipe startup with `Access to the path '\\.\pipe\LOCAL\dotnet_...' is denied`.

## Known risks

- SDK drift between .NET 10 and .NET 11 surfaces.
- The local .NET 11 preview SDK currently fails in this environment with Roslyn named-pipe access errors; validate `net11.0` in CI or after the preview SDK issue is resolved.
- NuGet package shape changing before Track 15 owns dry-run release planning.
- Cross-language fixture drift if Track 12 changes after the binding lands.
- Native FFI load checks remain blocked until Track 02 provides stable runtime artifacts.

## Integration notes

- Keep the implementation isolated to the C# package and local validation.
- Do not add NuGet publish automation or registry credentials here.
- Treat .NET 11 as preview until the runtime lane is stable in CI.

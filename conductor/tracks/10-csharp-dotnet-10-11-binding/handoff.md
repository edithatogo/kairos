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
`bindings/csharp/src/Kairo.ECS/NativeMethods.cs`
`bindings/csharp/src/Kairo.ECS/README.md`
`bindings/csharp/tests/Kairo.ECS.Tests/Kairo.ECS.Tests.csproj`
`bindings/csharp/tests/Kairo.ECS.Tests/PackageInfoTests.cs`
`bindings/csharp/tests/Kairo.ECS.Tests/DeterministicSchedulerTests.cs`
`bindings/csharp/tests/Kairo.ECS.Tests/ConformanceTests.cs`
`bindings/csharp/tests/Kairo.ECS.Tests/FfiSmokeTests.cs`
`bindings/csharp/tests/Kairo.ECS.Tests/NativeBindingTests.cs`
`bindings/csharp/README.md`
`conductor/tracks/10-csharp-dotnet-10-11-binding/spec.md`
`conductor/tracks/10-csharp-dotnet-10-11-binding/plan.md`
`conductor/tracks/10-csharp-dotnet-10-11-binding/test-matrix.md`
`conductor/tracks/10-csharp-dotnet-10-11-binding/handoff.md`
`conductor/tracks/10-csharp-dotnet-10-11-binding/risk-register.md`
`conductor/tracks.yaml`

## Contracts consumed

- Track 01 core type and scheduler contracts.
- Track 02 FFI contract for native library naming and explicit not-configured behavior.
- Track 12 conformance fixture contract shape for scheduler ordering coverage.
- Track 14 docs workflow only if the C# package adds broader user-facing docs.

## Contracts changed

- C# API surface adds `DeterministicScheduler`, `ScheduledEvent`, `EventStatus`, `NativeBinding`, `NativeBindingStatus`, `NativeMethods`, `KairoEcsEngineHandle`, `KairoEcsStats`, and `KairoEcsBuffer`.

## Tests added

- Project test coverage for exported C# entrypoints.
- Managed fixture parity checks for scheduler ordering by `time_ticks`, `priority`, and `sequence`.
- Native FFI smoke tests requiring explicit not-configured status when runtime assets are absent.
- Native FFI smoke tests for version, engine create/free, and empty step are gated behind explicit native binding configuration and are inconclusive when no runtime artifact exists.

## Validation commands

Executed from the repository root on 2026-05-07 for the stable .NET 10 lane:

```powershell
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet test bindings\csharp\Kairo.ECS.sln -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet build bindings\csharp\Kairo.ECS.sln -c Release -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet pack bindings\csharp\src\Kairo.ECS\Kairo.ECS.csproj -c Release -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
```

Results: tests passed 11/11 with 3 native smoke tests skipped as inconclusive because no native runtime library is configured, Release build succeeded with 0 warnings/errors, and local pack created `bindings/csharp/src/Kairo.ECS/bin/Release/Kairo.ECS.0.1.0-preview.1.nupkg`.

Additional isolated validation from `bindings/csharp` on 2026-05-07 for the stable .NET 10 lane:

```powershell
$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet build tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -f net10.0 --no-restore -v normal -p:UseSharedCompilation=false -m:1 -nr:false
$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet test tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -f net10.0 --no-restore -v normal -p:UseSharedCompilation=false -m:1 -nr:false
$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet restore tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -p:TargetFramework=net10.0 -v minimal
$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet build tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -f net10.0 -c Release --no-restore -v minimal -p:UseSharedCompilation=false -m:1 -nr:false
$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet pack src\Kairo.ECS\Kairo.ECS.csproj -c Release -v normal -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false -m:1 -nr:false
```

Results: Debug build passed with 0 warnings/errors, tests passed 11/11 with 3 native smoke tests skipped, Release build passed with 0 warnings/errors, and pack passed with the existing `Kairo.ECS.0.1.0-preview.1.nupkg` already up to date.

Experimental .NET 11 preview validation on 2026-05-07:

```powershell
C:\Users\60217257\scoop\apps\dotnet-sdk-preview\current\dotnet.exe restore bindings\csharp\tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -p:TargetFramework=net11.0 -v minimal
```

Result: preview restore passed. Preview build remains blocked before project compilation by Roslyn named-pipe access denial under the Scoop .NET 11 preview SDK.

## Known risks

- SDK drift between .NET 10 and .NET 11 surfaces.
- The local .NET 11 preview SDK currently fails in this environment with Roslyn named-pipe access errors; validate `net11.0` in CI or after the preview SDK issue is resolved.
- NuGet package shape changing before Track 15 owns dry-run release planning.
- Cross-language fixture drift if Track 12 changes after the binding lands.
- Native FFI load checks remain blocked until Track 02 provides stable runtime artifacts.

## Follow-up issues

- Validate the `net11.0` lane once stable .NET 11 SDK tooling is available in CI or locally.
- Run live native FFI smoke tests after Track 02 publishes a stable `kairo_ecs` runtime artifact for the current RID.
- Add AOT/trimming validation before Track 15 release dry-runs.

## Integration notes

- Keep the implementation isolated to the C# package and local validation.
- Do not add NuGet publish automation or registry credentials here.
- Treat .NET 11 as preview until the runtime lane is stable in CI.
- No release, registry, or remote publication side effects were performed.

## Last verified

2026-05-07

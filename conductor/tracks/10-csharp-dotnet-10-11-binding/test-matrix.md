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

## Local validation result — 2026-05-07

Stable .NET 10 lane passed after adding the low-level FFI declarations, `SafeHandle` wrapper, native smoke-test skip gate, and robust repo-level fixture discovery:

```powershell
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet test bindings\csharp\Kairo.ECS.sln -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet build bindings\csharp\Kairo.ECS.sln -c Release -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath='C:\Users\60217257\AppData\Local\Microsoft\dotnet\sdk\10.0.202\Sdks'; dotnet pack bindings\csharp\src\Kairo.ECS\Kairo.ECS.csproj -c Release -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
```

Results:

- `dotnet test`: passed 11/11 with 3 native smoke tests skipped as inconclusive because no native runtime library is configured.
- `dotnet build -c Release`: succeeded with 0 warnings and 0 errors on `net10.0`.
- `dotnet pack`: created `bindings/csharp/src/Kairo.ECS/bin/Release/Kairo.ECS.0.1.0-preview.1.nupkg`.

Additional isolated local validation on 2026-05-07:

- `$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet build tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -f net10.0 --no-restore -v normal -p:UseSharedCompilation=false -m:1 -nr:false`: passed with 0 warnings and 0 errors.
- `$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet test tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -f net10.0 --no-restore -v normal -p:UseSharedCompilation=false -m:1 -nr:false`: passed with 11 passed, 3 skipped, and 0 failed.
- `$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet build tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -f net10.0 -c Release --no-restore -v minimal -p:UseSharedCompilation=false -m:1 -nr:false`: passed with 0 warnings and 0 errors after a focused net10 restore.
- `$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet pack src\Kairo.ECS\Kairo.ECS.csproj -c Release -v normal -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false -m:1 -nr:false`: passed with the existing `Kairo.ECS.0.1.0-preview.1.nupkg` already up to date.
- `C:\Users\60217257\scoop\apps\dotnet-sdk-preview\current\dotnet.exe restore bindings\csharp\tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -p:TargetFramework=net11.0 -v minimal`: passed for the experimental net11 preview lane.

Blocked validation:

- Full `net11.0` build/test validation is experimental until stable SDK tooling is available in this environment or CI.
- The installed Scoop .NET 11 preview SDK restores `net11.0` assets but fails Roslyn compiler startup with `Access to the path '\\.\pipe\LOCAL\dotnet_...' is denied`.
- Live native FFI execution is blocked until Track 02 supplies a stable `kairo_ecs` runtime library.

## Review closeout validation — 2026-05-08

Worker C reran the focused Track 10 gates from the shared workspace:

```powershell
$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet test Kairo.ECS.sln -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet build Kairo.ECS.sln -c Release -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet test Kairo.ECS.sln --filter TestCategory=Conformance -m:1 -nr:false -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false
$env:MSBuildSDKsPath=$null; $env:DOTNET_CLI_TELEMETRY_OPTOUT='1'; dotnet pack src\Kairo.ECS\Kairo.ECS.csproj -c Release -v minimal -p:TargetFrameworks=net10.0 -p:UseSharedCompilation=false -m:1 -nr:false
node tests\conformance\track07_13_hardening_check.mjs
C:\Users\60217257\scoop\apps\dotnet-sdk-preview\current\dotnet.exe restore bindings\csharp\tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -p:TargetFramework=net11.0 -v minimal
C:\Users\60217257\scoop\apps\dotnet-sdk-preview\current\dotnet.exe test bindings\csharp\tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -f net11.0 --no-restore -v minimal -p:UseSharedCompilation=false -m:1 -nr:false
pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1
pwsh -NoProfile -File scripts\validate_track_no_skip_claims.ps1
```

Results:

- Stable `net10.0` test passed with 11 passed, 3 skipped native FFI tests, and 0 failed.
- Stable `net10.0` Release build passed with 0 warnings and 0 errors.
- Stable `net10.0` conformance-filter test passed with 3 passed and 0 failed.
- Stable `net10.0` pack passed and created `Kairo.ECS.0.1.0-preview.1.nupkg`.
- Track 07-13 hardening, Conductor phase-gate validation, and no-skip claim validation passed.
- `net11.0` preview restore passed inside the sandbox.
- `net11.0` preview test initially failed inside the sandbox before project compilation with the known Roslyn named-pipe access denial: `Access to the path '\\.\pipe\LOCAL\dotnet_...' is denied`.
- The same focused `net11.0` preview test passed outside the sandbox on 2026-05-08 with 11 passed, 3 native FFI tests skipped, and 0 failed:

```powershell
C:\Users\60217257\scoop\apps\dotnet-sdk-preview\current\dotnet.exe test bindings\csharp\tests\Kairo.ECS.Tests\Kairo.ECS.Tests.csproj -f net11.0 --no-restore -v minimal -p:UseSharedCompilation=false -m:1 -nr:false
```

Review closeout decision: Track 10 is `Done`; no waiver was required.

## Native resolver review fix — 2026-05-08

- Fixed: configured native-library status now aligns with actual P/Invoke loading by registering a `NativeLibrary` resolver for `kairo_ecs`.
- Passed: stable `net10.0` test, build, conformance-filter test, pack, Track 07-13 hardening, phase-gate validation, and no-skip claim validation.
- Blocked: `dotnet format` and `net11.0` preview test in this sandbox fail before project compilation with Roslyn named-pipe access denial. Formal local-environment waiver accepted for Track 10 closeout; retest in CI or a non-sandboxed SDK host.

## Future-surface controls

- Do not add NuGet publishing, signing, or registry credentials here.
- Do not widen into other language bindings or release-engineering surfaces.
- Do not extend beyond the C# binding and local package validation boundary.
- Stop at local compile/test/pack validation until Track 12 owns parity and Track 15 owns dry-runs, with Track 42 owning publication.

## CI command

```bash
dotnet test bindings/csharp/Kairo.ECS.sln && dotnet build bindings/csharp/Kairo.ECS.sln -c Release && dotnet pack bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj -c Release
```
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.

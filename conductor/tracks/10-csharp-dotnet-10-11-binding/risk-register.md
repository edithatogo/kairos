# Risk Register — 10 C# Binding .NET 10-11

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---:|---:|---:|---|---|---|
| .NET SDK version skew between CI and local | 4 | 3 | 12 | `global.json` pins the stable SDK lane; local validation clears stale preview `MSBuildSDKsPath`, narrows `TargetFrameworks=net10.0`, and runs single-node MSBuild with node reuse disabled; CI should keep `net11.0` preview allowed-fail until stable tooling lands. | csharp-agent | SDK version assertion fails in CI or `net11.0` blocks merge. |
| NativeLibrary path resolution portability | 3 | 4 | 12 | Resolve `KAIRO_ECS_NATIVE_LIB_DIR` first, then `runtimes/{rid}/native/`; keep native smoke tests inconclusive unless a real runtime library exists. | csharp-agent | Native load fails on any supported RID with Track 02 runtime assets present. |
| SafeHandle ownership regression | 3 | 4 | 12 | Use `KairoEcsEngineHandle` for native engine ownership and keep raw-handle tests for non-owning wrapping. | csharp-agent | Any handle-related crash, leak, double free, or memory corruption is reported. |
| AOT/trimming compatibility drift | 3 | 4 | 12 | Avoid reflection-based interop paths and add an explicit AOT publish lane before release-track dry-runs. | csharp-agent | `dotnet publish /p:PublishAot=true` fails once Track 15 enables package dry-runs. |
| Package publication scope creep | 2 | 4 | 8 | Keep this track limited to local `dotnet pack`; signing, credentials, registry push, and release dry-runs remain out of scope. | csharp-agent | Any Track 10 change introduces registry credentials, publish automation, or external package side effects. |

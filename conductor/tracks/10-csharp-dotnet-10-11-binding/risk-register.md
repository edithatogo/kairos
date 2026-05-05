# Risk Register — 10 C# Binding .NET 10-11

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| .NET SDK version skew between CI and local | 4 | 3 | 12 | global.json pins exact SDK version; CI matrix: `net10.0` (stable), `net11.0` (preview, allowed-fail); `dotnet --version` assertion in build script | csharp-agent | SDK version assertion fails in CI |
| NativeLibrary path resolution portability | 3 | 4 | 12 | Ship `runtimes/{rid}/native/` structure per RID; use `NativeLibrary.SetDllImportResolver` with fallback to `KAIRO_ECS_NATIVE_LIB_DIR` env var | csharp-agent | NativeLibrary load fails on any supported RID |
| nuget.org publishing delays | 4 | 3 | 12 | Use `nuget push --skip-duplicate` idempotency; verify resolution within 60 min via CI post-publish probe; maintain GitHub Packages registry as fallback | csharp-agent | Version unresolved on nuget.org >1hr after publish |
| SafeHandle<T> evolution in .NET 10/11 | 3 | 4 | 12 | Avoid `DangerousAddRef`/`DangerousRelease` except in interop layer; wrap in standard SafeHandle types; add `GC.KeepAlive` guard in `Dispose(bool)` paths | csharp-agent | Any handle-related crash or memory corruption reported |
| AOT compilation compatibility | 3 | 4 | 12 | Run `dotnet publish /p:PublishAot=true` in CI; mark binding assembly with appropriate trimming attributes; avoid reflection-based interop paths | csharp-agent | `dotnet publish /p:PublishAot=true` fails |

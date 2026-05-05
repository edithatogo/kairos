# Test Matrix — 10 C# Binding .NET 10-11

## Required tests

- `dotnet test bindings/csharp/Kairo.ECS.sln` for the C# binding project coverage.
- `dotnet build bindings/csharp/Kairo.ECS.sln -c Release` to validate compile-time compatibility across the supported SDKs.
- `dotnet test --filter Category=Conformance` or equivalent when Track 12 fixtures are wired in.
- `dotnet pack bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj -c Release` to validate NuGet package contents before any future registry work.
- `dotnet format --verify-no-changes` if the track owns C# source formatting rules.

## Future-surface controls

- Do not add NuGet publishing, signing, or registry credentials here.
- Do not widen into other language bindings or release-engineering surfaces.
- Do not extend beyond the C# binding and local package validation boundary.
- Stop at local compile/test/pack validation until Track 12 owns parity and Track 15 owns dry-runs.

## CI command

```bash
dotnet test bindings/csharp/Kairo.ECS.sln && dotnet build bindings/csharp/Kairo.ECS.sln -c Release && dotnet pack bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj -c Release
```


# C# Getting Started

## Prerequisites
- .NET SDK 10.0 or .NET 11.0 preview
- The KairoECS repository cloned

## Quickstart

### 1. Restore and build
```bash
cd bindings/csharp
dotnet restore Kairo.ECS.sln
dotnet build Kairo.ECS.sln -c Release
```

### 2. Verify the package surface
```csharp
using Kairo.ECS;
var info = PackageInfo.GetInfo();
Console.WriteLine(info);
```

### 3. Run tests
```bash
dotnet test Kairo.ECS.sln -c Release
```

## Package structure

| File | Purpose |
|---|---|
| `src/Kairo.ECS/Kairo.ECS.csproj` | Library project targeting `net10.0;net11.0` |
| `src/Kairo.ECS/PackageInfo.cs` | Reflection-based metadata class |
| `tests/Kairo.ECS.Tests/PackageInfoTests.cs` | Smoke and metadata tests |

## Target frameworks

- `net10.0` — stable lane
- `net11.0` — preview lane (requires .NET 11 SDK)

## Native library loading

Set `KAIRO_ECS_NATIVE_LIB_DIR` environment variable to the directory containing:
- `kairo_ecs.dll` (Windows)
- `libkairo_ecs.dylib` (macOS)
- `libkairo_ecs.so` (Linux)

## Next steps

- Read the [C# binding README](../../bindings/csharp/README.md)
- Explore the [NuGet packaging guide](../../packaging/README.md)

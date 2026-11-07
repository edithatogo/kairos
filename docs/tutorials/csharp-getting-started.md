# C# Getting Started

**Runtime support:** The managed scheduler preview is unsupported. Raw `NativeMethods` declarations target the Rust C ABI, but their availability does not prove a shipped native library or a complete managed model API.

Rust is the only supported simulation core. Follow [ADR 0010](../adr/0010-rust-only-execution.md), [installation](../install.md) and the [migration guide](../release/migration.md). The package checks below establish only their stated package/data scope.

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

Use the exact library identity and loading instructions in the binding README
and `NativeMethods` source. The native library and C ABI must match the installed
adapter's platform, architecture and version. Check actual engine creation,
scheduling, execution and handle disposal; a metadata check is insufficient.

## Next steps

- Read the [C# binding README](../../bindings/csharp/README.md)
- Explore the [NuGet packaging guide](../../packaging/README.md)

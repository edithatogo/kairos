# Kairo.ECS

This project hosts the C# binding surface for Kairo ECS.

The current preview slice is intentionally managed-only:

- `PackageInfo` exposes package metadata.
- `DeterministicScheduler` provides a deterministic event scheduler facade for
  binding and conformance tests.
- `NativeBinding` reports whether native FFI is configured without loading
  unmanaged code.

Native FFI remains not configured until `KAIRO_ECS_NATIVE_LIB_DIR` points to a
directory containing the platform library or future NuGet runtime assets are
packaged under `runtimes/{rid}/native/`.

Build:

```powershell
dotnet build ..\..\Kairo.ECS.sln
```

Test:

```powershell
dotnet test ..\..\Kairo.ECS.sln
```

Pack:

```powershell
dotnet pack .\Kairo.ECS.csproj -c Release
```

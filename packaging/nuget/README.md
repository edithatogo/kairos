# NuGet Packaging

Track 10 owns the local NuGet package shape for `Kairo.ECS`.

Current validation boundary:

```powershell
dotnet test bindings\csharp\Kairo.ECS.sln
dotnet build bindings\csharp\Kairo.ECS.sln -c Release
dotnet pack bindings\csharp\src\Kairo.ECS\Kairo.ECS.csproj -c Release
```

The package currently includes a managed deterministic scheduler facade and explicit native FFI configuration status. It does not ship native runtime assets yet.

Native FFI remains not configured unless one of these is present:

- `KAIRO_ECS_NATIVE_LIB_DIR` points to a directory containing the platform library.
- The package includes `runtimes/{rid}/native/{library}` for the active runtime identifier.

Track 15 owns future publish automation, signing, registry credentials, and release dry-runs.

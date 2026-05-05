using System.Runtime.InteropServices;

namespace Kairo.ECS;

/// <summary>
/// Resolves the optional native FFI package location without loading unmanaged code.
/// </summary>
public static class NativeBinding
{
    private const string NativeDirectoryVariable = "KAIRO_ECS_NATIVE_LIB_DIR";

    /// <summary>
    /// Gets the native library filename for the current platform.
    /// </summary>
    public static string PlatformLibraryName
    {
        get
        {
            if (OperatingSystem.IsWindows())
            {
                return "kairo_ecs.dll";
            }

            if (OperatingSystem.IsMacOS())
            {
                return "libkairo_ecs.dylib";
            }

            return "libkairo_ecs.so";
        }
    }

    /// <summary>
    /// Reports whether the native FFI library is configured.
    /// </summary>
    public static NativeBindingStatus GetStatus()
    {
        var configuredDirectory = Environment.GetEnvironmentVariable(NativeDirectoryVariable);
        if (!string.IsNullOrWhiteSpace(configuredDirectory))
        {
            var candidate = Path.Combine(configuredDirectory, PlatformLibraryName);
            if (File.Exists(candidate))
            {
                return new NativeBindingStatus(true, candidate, $"Resolved from {NativeDirectoryVariable}.");
            }

            return new NativeBindingStatus(false, null, $"{NativeDirectoryVariable} is set, but '{candidate}' does not exist.");
        }

        var runtimeCandidate = Path.Combine(
            AppContext.BaseDirectory,
            "runtimes",
            RuntimeInformation.RuntimeIdentifier,
            "native",
            PlatformLibraryName);

        if (File.Exists(runtimeCandidate))
        {
            return new NativeBindingStatus(true, runtimeCandidate, "Resolved from package runtime assets.");
        }

        return new NativeBindingStatus(
            false,
            null,
            $"Native FFI is not configured. Set {NativeDirectoryVariable} or include runtimes/{RuntimeInformation.RuntimeIdentifier}/native/{PlatformLibraryName}.");
    }
}

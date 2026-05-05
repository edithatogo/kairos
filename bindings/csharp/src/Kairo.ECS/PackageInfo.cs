using System.Reflection;

namespace Kairo.ECS;

/// <summary>
/// Exposes package metadata embedded in the Kairo ECS assembly.
/// </summary>
public static class PackageInfo
{
    /// <summary>
    /// Gets the package identifier embedded in assembly metadata.
    /// </summary>
    public static string PackageId => ReadMetadata("PackageId");

    /// <summary>
    /// Gets the package version embedded in assembly metadata.
    /// </summary>
    public static string PackageVersion => ReadMetadata("PackageVersion");

    /// <summary>
    /// Returns a compact package identifier and version string.
    /// </summary>
    public static string Describe() => $"{PackageId} {PackageVersion}";

    private static string ReadMetadata(string key)
    {
        var metadata = typeof(PackageInfo)
            .Assembly
            .GetCustomAttributes<AssemblyMetadataAttribute>()
            .FirstOrDefault(attribute => attribute.Key == key);

        return metadata?.Value
            ?? throw new InvalidOperationException($"Assembly metadata '{key}' was not found.");
    }
}

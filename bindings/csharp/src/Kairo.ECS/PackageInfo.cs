using System.Reflection;

namespace Kairo.ECS;

public static class PackageInfo
{
    public static string PackageId => ReadMetadata("PackageId");

    public static string PackageVersion => ReadMetadata("PackageVersion");

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

using System.Reflection;
using Kairo.ECS;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Kairo.ECS.Tests;

[TestClass]
public sealed class PackageInfoTests
{
    [TestMethod]
    [TestCategory("Smoke")]
    public void Package_identity_matches_the_project_metadata()
    {
        Assert.AreEqual("Kairo.ECS", PackageInfo.PackageId);
        Assert.AreEqual("0.1.0-preview.1", PackageInfo.PackageVersion);
        Assert.AreEqual("Kairo.ECS 0.1.0-preview.1", PackageInfo.Describe());
        Assert.AreEqual("Kairo.ECS", typeof(PackageInfo).Assembly.GetName().Name);
    }

    [TestMethod]
    [TestCategory("Smoke")]
    public void Assembly_metadata_includes_package_identity_values()
    {
        var metadata = typeof(PackageInfo)
            .Assembly
            .GetCustomAttributes<AssemblyMetadataAttribute>()
            .ToDictionary(attribute => attribute.Key, attribute => attribute.Value);

        Assert.AreEqual("Kairo.ECS", metadata["PackageId"]);
        Assert.AreEqual("0.1.0-preview.1", metadata["PackageVersion"]);
    }
}

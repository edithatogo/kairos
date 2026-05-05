using Kairo.ECS;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Kairo.ECS.Tests;

[TestClass]
public sealed class NativeBindingTests
{
    [TestMethod]
    [TestCategory("Smoke")]
    public void Native_binding_reports_not_configured_without_runtime_assets()
    {
        var previous = Environment.GetEnvironmentVariable("KAIRO_ECS_NATIVE_LIB_DIR");
        try
        {
            Environment.SetEnvironmentVariable("KAIRO_ECS_NATIVE_LIB_DIR", null);

            var status = NativeBinding.GetStatus();

            Assert.IsFalse(status.IsConfigured);
            Assert.IsNull(status.LibraryPath);
            StringAssert.Contains(status.Reason, "Native FFI is not configured");
        }
        finally
        {
            Environment.SetEnvironmentVariable("KAIRO_ECS_NATIVE_LIB_DIR", previous);
        }
    }

    [TestMethod]
    [TestCategory("Smoke")]
    public void Native_binding_uses_platform_specific_library_name()
    {
        var libraryName = NativeBinding.PlatformLibraryName;

        Assert.IsTrue(
            libraryName is "kairo_ecs.dll" or "libkairo_ecs.dylib" or "libkairo_ecs.so",
            $"Unexpected native library name: {libraryName}");
    }
}

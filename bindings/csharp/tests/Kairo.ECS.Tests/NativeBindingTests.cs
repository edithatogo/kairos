using System.Reflection;
using Kairo.ECS;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Kairo.ECS.Tests;

[TestClass]
[DoNotParallelize]
public sealed class NativeBindingTests
{
    [TestMethod]
    [TestCategory("Smoke")]
    public void Native_binding_reports_not_configured_without_runtime_assets()
    {
        var previous = Environment.GetEnvironmentVariable("KAIRO_ECS_NATIVE_LIB_DIR");
        var previousLibrary = Environment.GetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY");
        try
        {
            Environment.SetEnvironmentVariable("KAIRO_ECS_NATIVE_LIB_DIR", null);
            Environment.SetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY", null);

            var status = NativeBinding.GetStatus();

            Assert.IsFalse(status.IsConfigured);
            Assert.IsNull(status.LibraryPath);
            StringAssert.Contains(status.Reason, "Native FFI is not configured");
        }
        finally
        {
            Environment.SetEnvironmentVariable("KAIRO_ECS_NATIVE_LIB_DIR", previous);
            Environment.SetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY", previousLibrary);
        }
    }

    [TestMethod]
    public void Explicit_library_path_reports_discovery_only_and_rejects_relative_paths()
    {
        var previous = Environment.GetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY");
        var path = Path.GetTempFileName();
        try
        {
            Environment.SetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY", "relative-library.so");
            Assert.IsFalse(NativeBinding.GetStatus().IsConfigured);
            Environment.SetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY", path);
            var status = NativeBinding.GetStatus();
            Assert.IsTrue(status.IsConfigured);
            Assert.AreEqual(path, status.LibraryPath);
            StringAssert.Contains(status.Reason, "not yet verified");
        }
        finally
        {
            Environment.SetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY", previous);
            File.Delete(path);
        }
    }

    [TestMethod]
    public void Invalid_explicit_library_cannot_fall_through_to_ambient_resolution()
    {
        var previous = Environment.GetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY");
        try
        {
            var missing = Path.Combine(Path.GetTempPath(), "missing-rust-" + Guid.NewGuid().ToString("N"));
            Environment.SetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY", missing);
            // Invoke the actual resolver so a library cached by another native
            // test cannot mask this failure boundary.
            var resolver = typeof(NativeMethods).GetMethod("ResolveNativeLibrary", BindingFlags.NonPublic | BindingFlags.Static);
            Assert.IsNotNull(resolver);
            var exception = Assert.ThrowsExactly<TargetInvocationException>(() =>
                resolver!.Invoke(null, new object?[] { "kairo_ecs", typeof(NativeMethods).Assembly, null }));
            Assert.IsTrue(exception.InnerException is DllNotFoundException);
            StringAssert.Contains(exception.InnerException!.Message, "KAIRO_ECS_FFI_LIBRARY");
        }
        finally
        {
            Environment.SetEnvironmentVariable("KAIRO_ECS_FFI_LIBRARY", previous);
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

    [TestMethod]
    [TestCategory("Smoke")]
    public void Safe_handle_wraps_raw_engine_handle_value()
    {
        using var handle = KairoEcsEngineHandle.FromRawHandle(42, ownsHandle: false);

        Assert.IsFalse(handle.IsInvalid);
        Assert.AreEqual(42ul, handle.RawHandle);
    }
}

using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Kairo.ECS.Tests;

[TestClass]
public sealed class FfiSmokeTests
{
    [TestMethod]
    [TestCategory("Smoke")]
    public void FfiVersion_ReturnsNonZero()
    {
        RequireNativeBinding();

        var version = NativeMethods.kairo_ecs_ffi_version();
        Assert.AreNotEqual(0u, version);
    }

    [TestMethod]
    [TestCategory("Smoke")]
    public void EngineCreateFree_NoCrash()
    {
        RequireNativeBinding();

        var engine = NativeMethods.kairo_ecs_engine_new();
        Assert.AreNotEqual(0ul, engine);
        using var safeHandle = KairoEcsEngineHandle.FromRawHandle(engine);
        Assert.IsFalse(safeHandle.IsInvalid);
    }

    [TestMethod]
    [TestCategory("Smoke")]
    public void Step_Empty_ReturnsOk()
    {
        RequireNativeBinding();

        var engine = NativeMethods.kairo_ecs_engine_new();
        var result = NativeMethods.kairo_ecs_step(engine);
        Assert.AreEqual(0, result);
        NativeMethods.kairo_ecs_engine_free(engine);
    }

    private static void RequireNativeBinding()
    {
        var status = NativeBinding.GetStatus();
        if (!status.IsConfigured)
        {
            Assert.Inconclusive(status.Reason);
        }
    }
}

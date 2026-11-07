using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Kairo.ECS.Tests;

[TestClass]
public sealed class FfiSmokeTests
{
    [TestMethod]
    [TestCategory("Smoke")]
    [TestCategory("Native")]
    public void FfiVersion_MatchesSupportedAbi()
    {
        RequireNativeBinding();

        var version = NativeMethods.kairo_ecs_ffi_version();
        Assert.AreEqual(1u, version);
    }

    [TestMethod]
    [TestCategory("Smoke")]
    [TestCategory("Native")]
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
    [TestCategory("Native")]
    public void Step_Empty_ReturnsOk()
    {
        RequireNativeBinding();

        var engine = NativeMethods.kairo_ecs_engine_new();
        Assert.AreNotEqual(0ul, engine);
        using var safeHandle = KairoEcsEngineHandle.FromRawHandle(engine);
        var result = NativeMethods.kairo_ecs_step(engine);
        Assert.AreEqual(0, result);
        Assert.AreEqual(0ul, NativeMethods.kairo_ecs_stats(engine).DispatchedEvents);
    }

    [TestMethod]
    [TestCategory("Native")]
    public void RustEngineSchedulesCancelsDispatchesAndResets()
    {
        RequireNativeBinding();
        var engine = NativeMethods.kairo_ecs_engine_new();
        Assert.AreNotEqual(0ul, engine);
        using var safeHandle = KairoEcsEngineHandle.FromRawHandle(engine);
        var first = NativeMethods.kairo_ecs_schedule_at(engine, 5, -1, 7);
        var cancelled = NativeMethods.kairo_ecs_schedule_at(engine, 3, 0, 8);
        Assert.AreNotEqual(0ul, first);
        Assert.AreNotEqual(0ul, cancelled);
        Assert.AreEqual(0, NativeMethods.kairo_ecs_cancel_event(engine, cancelled));
        Assert.AreEqual(0, NativeMethods.kairo_ecs_run_for(engine, 2));
        var stats = NativeMethods.kairo_ecs_stats(engine);
        Assert.AreEqual(5ul, stats.NowTicks);
        Assert.AreEqual(2ul, stats.ScheduledEvents);
        Assert.AreEqual(1ul, stats.CancelledEvents);
        Assert.AreEqual(1ul, stats.DispatchedEvents);
        Assert.AreEqual(0ul, stats.PendingEvents);
        Assert.AreEqual(0, NativeMethods.kairo_ecs_engine_reset(engine));
        Assert.AreEqual(0ul, NativeMethods.kairo_ecs_stats(engine).ScheduledEvents);
    }

    private static void RequireNativeBinding()
    {
        var status = NativeBinding.GetStatus();
        if (!status.IsConfigured)
        {
            if (Environment.GetEnvironmentVariable("KAIRO_ECS_REQUIRE_NATIVE") == "1")
            {
                Assert.Fail("Required Rust native integration is unavailable: " + status.Reason);
            }
            Assert.Inconclusive(status.Reason + " This is not native acceptance evidence. Set KAIRO_ECS_REQUIRE_NATIVE=1 in native CI.");
        }
    }
}

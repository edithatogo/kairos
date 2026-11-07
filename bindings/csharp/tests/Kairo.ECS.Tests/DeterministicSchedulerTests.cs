using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Kairo.ECS.Tests;

[TestClass]
public sealed class DeterministicSchedulerTests
{
    [TestMethod]
    [TestCategory("NativeRequired")]
    public void ManagedSchedulerFailsBeforeProducingAnySimulationState()
    {
        var exception = Assert.ThrowsExactly<NotSupportedException>(() => new DeterministicScheduler());
        StringAssert.Contains(exception.Message, "Rust");
    }
}

using Kairo.ECS;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Kairo.ECS.Tests;

[TestClass]
public sealed class DeterministicSchedulerTests
{
    [TestMethod]
    [TestCategory("Conformance")]
    public void Step_dispatches_by_time_priority_and_sequence()
    {
        var scheduler = new DeterministicScheduler();
        var third = scheduler.ScheduleAt(20, "third", priority: 0);
        var second = scheduler.ScheduleAt(10, "second", priority: 10);
        var first = scheduler.ScheduleAt(10, "first", priority: -1);
        var fourth = scheduler.ScheduleAt(20, "fourth", priority: 0);

        var dispatched = scheduler.RunFor(4);

        CollectionAssert.AreEqual(
            new[] { first.EventId, second.EventId, third.EventId, fourth.EventId },
            dispatched.Select(item => item.EventId).ToArray());
        Assert.IsTrue(dispatched.All(item => item.Status == EventStatus.Dispatched));
        Assert.AreEqual(20, scheduler.CurrentTimeTicks);
    }

    [TestMethod]
    [TestCategory("Conformance")]
    public void Cancelled_event_is_not_dispatched_but_stays_visible_in_snapshot()
    {
        var scheduler = new DeterministicScheduler();
        var cancelled = scheduler.ScheduleAt(5, "cancelled");
        var active = scheduler.ScheduleAt(10, "active");

        Assert.IsTrue(scheduler.Cancel(cancelled.EventId));
        Assert.IsFalse(scheduler.Cancel(cancelled.EventId));

        var dispatched = scheduler.RunFor(2);
        var snapshot = scheduler.Snapshot();

        Assert.AreEqual(active.EventId, dispatched.Single().EventId);
        Assert.AreEqual(EventStatus.Cancelled, snapshot.Single(item => item.EventId == cancelled.EventId).Status);
    }

    [TestMethod]
    [TestCategory("Conformance")]
    public void Run_until_advances_time_without_unbounded_loop()
    {
        var scheduler = new DeterministicScheduler();
        scheduler.ScheduleAfter(3, "early");
        scheduler.ScheduleAfter(9, "late");

        var dispatched = scheduler.RunUntil(5);

        Assert.AreEqual("early", dispatched.Single().EventKind);
        Assert.AreEqual(5, scheduler.CurrentTimeTicks);
        Assert.AreEqual("late", scheduler.Step()?.EventKind);
        Assert.AreEqual(9, scheduler.CurrentTimeTicks);
    }
}

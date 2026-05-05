namespace Kairo.ECS;

/// <summary>
/// Immutable event record exposed by the managed scheduler facade.
/// </summary>
/// <param name="EventId">Monotonic event identifier assigned by the scheduler.</param>
/// <param name="TimeTicks">Fixed tick-based simulation time.</param>
/// <param name="Priority">Lower values dispatch first at the same simulation time.</param>
/// <param name="Sequence">Monotonic insertion sequence used as the final ordering key.</param>
/// <param name="EventKind">Scheduler-visible event classification.</param>
/// <param name="Status">Current event lifecycle status.</param>
/// <param name="PayloadRef">Optional external payload reference.</param>
public sealed record ScheduledEvent(
    ulong EventId,
    long TimeTicks,
    int Priority,
    ulong Sequence,
    string EventKind,
    EventStatus Status,
    string? PayloadRef);

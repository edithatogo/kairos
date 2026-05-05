using System.Collections.ObjectModel;

namespace Kairo.ECS;

/// <summary>
/// Managed deterministic scheduler facade for binding and conformance tests.
/// </summary>
public sealed class DeterministicScheduler
{
    private readonly List<ScheduledEvent> events = [];
    private ulong nextEventId = 1;
    private ulong nextSequence;

    /// <summary>
    /// Gets the scheduler's current fixed tick time.
    /// </summary>
    public long CurrentTimeTicks { get; private set; }

    /// <summary>
    /// Schedules an event at an absolute simulation time.
    /// </summary>
    public ScheduledEvent ScheduleAt(long timeTicks, string eventKind, int priority = 0, string? payloadRef = null)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(eventKind);

        if (timeTicks < CurrentTimeTicks)
        {
            throw new ArgumentOutOfRangeException(nameof(timeTicks), "Events cannot be scheduled before the current simulation time.");
        }

        var scheduled = new ScheduledEvent(
            nextEventId++,
            timeTicks,
            priority,
            nextSequence++,
            eventKind,
            EventStatus.Scheduled,
            payloadRef);

        events.Add(scheduled);
        return scheduled;
    }

    /// <summary>
    /// Schedules an event after a non-negative duration from the current simulation time.
    /// </summary>
    public ScheduledEvent ScheduleAfter(long durationTicks, string eventKind, int priority = 0, string? payloadRef = null)
    {
        if (durationTicks < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(durationTicks), "Event duration must be non-negative.");
        }

        return ScheduleAt(checked(CurrentTimeTicks + durationTicks), eventKind, priority, payloadRef);
    }

    /// <summary>
    /// Cancels a scheduled event.
    /// </summary>
    public bool Cancel(ulong eventId)
    {
        var index = events.FindIndex(item => item.EventId == eventId);
        if (index < 0 || events[index].Status != EventStatus.Scheduled)
        {
            return false;
        }

        events[index] = events[index] with { Status = EventStatus.Cancelled };
        return true;
    }

    /// <summary>
    /// Dispatches the next event by the core contract ordering: time, priority, sequence.
    /// </summary>
    public ScheduledEvent? Step()
    {
        var next = events
            .Where(item => item.Status == EventStatus.Scheduled)
            .OrderBy(item => item.TimeTicks)
            .ThenBy(item => item.Priority)
            .ThenBy(item => item.Sequence)
            .FirstOrDefault();

        if (next is null)
        {
            return null;
        }

        CurrentTimeTicks = next.TimeTicks;
        var dispatched = next with { Status = EventStatus.Dispatched };
        Replace(next.EventId, dispatched);
        return dispatched;
    }

    /// <summary>
    /// Dispatches up to <paramref name="maxEvents"/> events.
    /// </summary>
    public IReadOnlyList<ScheduledEvent> RunFor(int maxEvents)
    {
        if (maxEvents < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(maxEvents), "Maximum event count must be non-negative.");
        }

        var dispatched = new List<ScheduledEvent>(maxEvents);
        for (var index = 0; index < maxEvents; index++)
        {
            var next = Step();
            if (next is null)
            {
                break;
            }

            dispatched.Add(next);
        }

        return dispatched;
    }

    /// <summary>
    /// Dispatches scheduled events with simulation time less than or equal to the supplied limit.
    /// </summary>
    public IReadOnlyList<ScheduledEvent> RunUntil(long timeLimitTicks)
    {
        if (timeLimitTicks < CurrentTimeTicks)
        {
            throw new ArgumentOutOfRangeException(nameof(timeLimitTicks), "Time limit cannot be before the current simulation time.");
        }

        var dispatched = new List<ScheduledEvent>();
        while (PeekNextScheduled() is { } next && next.TimeTicks <= timeLimitTicks)
        {
            dispatched.Add(Step()!);
        }

        CurrentTimeTicks = Math.Max(CurrentTimeTicks, timeLimitTicks);
        return dispatched;
    }

    /// <summary>
    /// Returns a stable snapshot of all events for test and binding inspection.
    /// </summary>
    public IReadOnlyList<ScheduledEvent> Snapshot() => new ReadOnlyCollection<ScheduledEvent>(events.ToList());

    private ScheduledEvent? PeekNextScheduled() => events
        .Where(item => item.Status == EventStatus.Scheduled)
        .OrderBy(item => item.TimeTicks)
        .ThenBy(item => item.Priority)
        .ThenBy(item => item.Sequence)
        .FirstOrDefault();

    private void Replace(ulong eventId, ScheduledEvent replacement)
    {
        var index = events.FindIndex(item => item.EventId == eventId);
        if (index < 0)
        {
            throw new InvalidOperationException($"Event '{eventId}' was not found.");
        }

        events[index] = replacement;
    }
}

namespace Kairo.ECS;

/// <summary>
/// Retired managed scheduler. Execution requires the Rust core through NativeMethods.
/// This surface remains only to report the breaking change explicitly.
/// </summary>
public sealed class DeterministicScheduler
{
    private static NotSupportedException Retired() => new(
        "The managed scheduler is retired. Use the Rust C ABI through NativeMethods; no C# core or fallback is supported.");

    /// <summary>Fails because the managed scheduler is no longer supported.</summary>
    public DeterministicScheduler() => throw Retired();

    /// <summary>Requires a Rust-backed engine adapter.</summary>
    public long CurrentTimeTicks => throw Retired();

    /// <summary>Requires a Rust-backed engine adapter.</summary>
    public ScheduledEvent ScheduleAt(long timeTicks, string eventKind, int priority = 0, string? payloadRef = null) => throw Retired();

    /// <summary>Requires a Rust-backed engine adapter.</summary>
    public ScheduledEvent ScheduleAfter(long durationTicks, string eventKind, int priority = 0, string? payloadRef = null) => throw Retired();

    /// <summary>Requires a Rust-backed engine adapter.</summary>
    public bool Cancel(ulong eventId) => throw Retired();

    /// <summary>Requires a Rust-backed engine adapter.</summary>
    public ScheduledEvent? Step() => throw Retired();

    /// <summary>Requires a Rust-backed engine adapter.</summary>
    public IReadOnlyList<ScheduledEvent> RunFor(int maxEvents) => throw Retired();

    /// <summary>Requires a Rust-backed engine adapter.</summary>
    public IReadOnlyList<ScheduledEvent> RunUntil(long timeLimitTicks) => throw Retired();

    /// <summary>Requires a Rust-backed engine adapter.</summary>
    public IReadOnlyList<ScheduledEvent> Snapshot() => throw Retired();
}

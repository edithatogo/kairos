namespace Kairo.ECS;

/// <summary>
/// Represents the lifecycle status for an event in event-log data.
/// </summary>
public enum EventStatus
{
    /// <summary>
    /// The event has been scheduled and has not yet been dispatched.
    /// </summary>
    Scheduled,

    /// <summary>
    /// The event was dispatched by the scheduler.
    /// </summary>
    Dispatched,

    /// <summary>
    /// The event was cancelled before dispatch.
    /// </summary>
    Cancelled,
}

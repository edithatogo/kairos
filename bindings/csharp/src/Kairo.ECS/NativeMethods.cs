using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace Kairo.ECS;

/// <summary>
/// Raw P/Invoke declarations for the Kairo ECS native FFI library.
/// </summary>
public static class NativeMethods
{
    private const string DllName = "kairo_ecs";

    /// <summary>
    /// Gets the native FFI contract version.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern uint kairo_ecs_ffi_version();

    /// <summary>
    /// Allocates a native ECS engine and returns its opaque handle.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern ulong kairo_ecs_engine_new();

    /// <summary>
    /// Frees a native ECS engine handle.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int kairo_ecs_engine_free(ulong handle);

    /// <summary>
    /// Resets an existing native ECS engine.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int kairo_ecs_engine_reset(ulong handle);

    /// <summary>
    /// Gets the current engine clock in ticks.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern ulong kairo_ecs_engine_current_time(ulong handle);

    /// <summary>
    /// Schedules an event at an absolute engine tick.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern ulong kairo_ecs_schedule_at(ulong handle, ulong atTicks, int priority, uint kind);

    /// <summary>
    /// Schedules an event after the supplied tick delay.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern ulong kairo_ecs_schedule_after(ulong handle, ulong afterTicks, int priority, uint kind);

    /// <summary>
    /// Cancels a scheduled native event by handle.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int kairo_ecs_cancel_event(ulong handle, ulong eventHandle);

    /// <summary>
    /// Dispatches one pending event from the native engine.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int kairo_ecs_step(ulong handle);

    /// <summary>
    /// Runs the native engine for up to the supplied event count.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int kairo_ecs_run_for(ulong handle, ulong maxEvents);

    /// <summary>
    /// Runs the native engine until the supplied time limit.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int kairo_ecs_run_until(ulong handle, ulong timeLimitTicks);

    /// <summary>
    /// Runs the native engine until either the time limit or event count is reached.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern int kairo_ecs_run_until_or_for(ulong handle, ulong timeLimitTicks, ulong maxEvents);

    /// <summary>
    /// Gets the current native engine statistics snapshot.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern KairoEcsStats kairo_ecs_stats(ulong handle);

    /// <summary>
    /// Gets the most recent native error message pointer, if any.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr kairo_ecs_last_error_message();

    /// <summary>
    /// Flushes pending telemetry to an IPC buffer owned by the native library.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern KairoEcsBuffer kairo_ecs_telemetry_flush_ipc(ulong handle);

    /// <summary>
    /// Frees a native buffer returned by the ECS FFI.
    /// </summary>
    [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
    public static extern void kairo_ecs_buffer_free(KairoEcsBuffer buffer);
}

/// <summary>
/// SafeHandle wrapper for an opaque native Kairo ECS engine handle.
/// </summary>
public sealed class KairoEcsEngineHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    private KairoEcsEngineHandle(bool ownsHandle)
        : base(ownsHandle)
    {
    }

    /// <summary>
    /// Creates a managed SafeHandle from a raw native handle value.
    /// </summary>
    public static KairoEcsEngineHandle FromRawHandle(ulong rawHandle, bool ownsHandle = true)
    {
        var safeHandle = new KairoEcsEngineHandle(ownsHandle);
        safeHandle.SetHandle((IntPtr)checked((long)rawHandle));
        return safeHandle;
    }

    /// <summary>
    /// Gets the raw opaque handle value for P/Invoke calls.
    /// </summary>
    public ulong RawHandle => checked((ulong)handle.ToInt64());

    /// <summary>
    /// Releases the native ECS engine handle.
    /// </summary>
    protected override bool ReleaseHandle() => NativeMethods.kairo_ecs_engine_free(RawHandle) == 0;
}

/// <summary>
/// Stats snapshot returned by <see cref="NativeMethods.kairo_ecs_stats"/>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct KairoEcsStats
{
    /// <summary>
    /// Current engine clock in ticks.
    /// </summary>
    public ulong NowTicks;

    /// <summary>
    /// Total number of events scheduled by the engine.
    /// </summary>
    public ulong ScheduledEvents;

    /// <summary>
    /// Total number of events dispatched by the engine.
    /// </summary>
    public ulong DispatchedEvents;

    /// <summary>
    /// Total number of cancelled events.
    /// </summary>
    public ulong CancelledEvents;

    /// <summary>
    /// Current number of pending events.
    /// </summary>
    public ulong PendingEvents;
}

/// <summary>
/// Buffer descriptor returned by <see cref="NativeMethods.kairo_ecs_telemetry_flush_ipc"/>.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct KairoEcsBuffer
{
    /// <summary>
    /// Pointer to the native buffer data.
    /// </summary>
    public IntPtr Data;

    /// <summary>
    /// Length of the native buffer in bytes.
    /// </summary>
    public UIntPtr Length;
}

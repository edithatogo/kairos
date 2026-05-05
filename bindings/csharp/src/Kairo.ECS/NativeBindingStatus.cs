namespace Kairo.ECS;

/// <summary>
/// Describes whether the native Kairo ECS FFI library is configured for this process.
/// </summary>
/// <param name="IsConfigured">True when a native library candidate can be resolved.</param>
/// <param name="LibraryPath">Resolved native library path, when configured.</param>
/// <param name="Reason">Human-readable configuration status.</param>
public sealed record NativeBindingStatus(bool IsConfigured, string? LibraryPath, string Reason);

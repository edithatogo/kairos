# ADR 0017 Apple Metal particle residency

## Status

This decision applies to the optional `kairo-ecs-gpu` Metal particle
backend using wgpu 30.0.1.

## Decision

The hardware-only constructor requires an Apple integrated Metal adapter
reported by wgpu. ABM uploads replace the particle buffer and accept an
empty particle set. A step allocates and sends its 16-byte parameters,
waits on the native queue and leaves the particles on the device. An
explicit download stages a copy in its own buffer. Uploads check the
device's binding, dispatch and allocation limits before replacing state.

Native allocation and validation use wgpu error scopes. A failed mutating
dispatch leaves the particle state indeterminate until a successful
upload. Concurrent readers wait for their own map completion. Physical
device-loss recovery remains outside the supported evidence.

## Compatibility

Metal upload and step metrics count GPU transfers and workgroups. Host
DES updates through the general constructor report zero device activity.
The hardware-only constructor rejects DES. The runtime owns device
storage and provides explicit copies. A clone keeps the existing CPU
snapshot behaviour, including the current native particle state. Clone
can panic if readback fails. Use the fallible state download when a
caller needs to handle readback errors.

## Evidence

The independent scalar motion fixture spans two workgroups. It exercises
same-device replay, empty replacement, concurrent readers and clone
isolation. The native error case rejects a buffer binding and verifies
reupload recovery. The Apple M1 Max timings record end-to-end CPU and
Metal costs for the same particle workloads.
[S11 qualification](../../conductor/execution/scientific/s11-completion.md).

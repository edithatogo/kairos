---
title: C#
description: .NET 10/11 P/Invoke and SafeHandle bindings for KairoECS.
---

The C# binding (`bindings/csharp/`) targets `.NET 10` and `.NET 11` using `LibraryImport` / `P/Invoke` with `SafeHandle` lifetime management and Apache Arrow C Data Interface interop.

## .NET API Surface

| Type / Method | Purpose |
| --- | --- |
| `KairoSimulationHandle` | `SafeHandle` wrapper around native `kairo_sim_t*` ensuring deterministic disposal |
| `KairoSimulation.Create(ulong seed)` | Create a new deterministic simulation world |
| `KairoSimulation.Step(ulong steps)` | Step the scheduler forward and return current `SimTime` |
| `KairoSimulation.ExportArrowStream()` | Export zero-copy Arrow `RecordBatch` buffers to `Apache.Arrow` |

## Example

```csharp
using KairoEcs;

using var sim = KairoSimulation.Create(seed: 42UL);
ulong simTime = sim.Step(100UL);
ulong digest = sim.StateDigest();
Console.WriteLine($"Time={simTime}, Digest=0x{digest:X16}");
```

Canonical sources:

- `bindings/csharp/README.md`
- `docs/tutorials/csharp-getting-started.md`

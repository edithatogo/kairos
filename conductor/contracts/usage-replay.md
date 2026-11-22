# Committed staged usage under optimistic replay

Export committed occupancy only. A Time Warp fossil-collection cutoff is
exclusive. Earlier events are irrevocable and events at the cutoff may still
change. Require a usage window's end to be strictly earlier than the cutoff
because a release exactly at the endpoint is part of the exported provenance.

`OptimisticRuntime::committed_snapshot` returns an opaque process snapshot and
the cutoff from the same runtime. Use the checkpoint before the first
retained speculative event, or the live process checkpoint when no
speculative event remains. The caller of fossil collection must establish the
safe global bound for its execution mode.

Enable the DES `optimistic-usage` feature and keep the complete staged
allocator, including its scheduler, runtime identity, pending actions and
usage state, in the process's `StagedResourceCheckpoint`. The adapter accepts
this checkpoint through the committed process snapshot. Before exporting it
checks that no staged action is still due through the end of the window. It
returns the existing occupancy window with the producing LP and cutoff.
Window construction is read-only and retains causal admission and release
IDs. Scope usage identity to the producing LP and keep speculative handles
inside the process checkpoint.

The replay fixture admits usage at tick 2 and speculatively releases at tick
10. A straggler moves the release to tick 6. After fossil collection, export a
window ending at tick 8, verify four occupied unit-ticks and compare the result
across a later cutoff. Also exercise an unreplayed future release in retained
history, an endpoint at the cutoff and an unprocessed staged event.

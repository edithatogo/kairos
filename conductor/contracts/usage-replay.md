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

Enable the DES `optimistic-usage` feature and capture the staged allocator
with `StagedResourceRuntime::checkpoint`. Include its `StagedResourceCheckpoint`
in the process snapshot and implement `AsRef<StagedResourceCheckpoint>` on a
composite snapshot. Restore that checkpoint with the rest of the process state.

Run all allocation actions due through the observation end and call
`seal_usage_through` inside the model's event handler. This also covers quiet
periods beyond the allocator's last dispatch. The seal prevents new actions
through that tick and rolls back with the allocator. `committed_staged_usage`
requires this seal to be present in the committed snapshot and the observation
end to be strictly before its cutoff.

The exporter returns the existing occupancy window with the producing LP and
cutoff. Window construction is read-only and retains causal admission and
release IDs. Scope usage identity to the producing LP and keep speculative
handles inside the process checkpoint.

The replay fixture admits usage at tick 2 and speculatively releases at tick
10. A straggler moves the release to tick 6. After fossil collection, export a
window ending at tick 8 and verify four occupied unit-ticks and exact agreement
with canonical event order. A later cutoff preserves the same window. The
fixture also checks censoring of a future release, rejection at the cutoff,
and rejection of a seal when an unprocessed action is due.

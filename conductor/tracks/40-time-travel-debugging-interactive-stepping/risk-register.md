# Risk Register — 40 Time-Travel Debugging & Interactive Stepping

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Trace file size becomes unmanageable for 10M+ event runs, exceeding storage or memory limits | 4 | 4 | 16 | Delta encoding with periodic full snapshots; configurable snapshot interval; compression (zstd) on serialized trace; streaming trace writer to disk to limit memory footprint | timetravel-agent | Trace file for 10M-event benchmark exceeds 10 GB or causes OOM during record |
| Snapshot storage overhead per tick bloats memory if full-state snapshots are taken too frequently | 3 | 4 | 12 | Sparse snapshots (every N ticks) with deltas in between; adaptive snapshot frequency based on state change rate; snapshot compression | timetravel-agent | Memory usage during trace recording exceeds 2x baseline simulation memory |
| Forward/backward parity verification fails silently, producing divergent state on replay | 2 | 5 | 10 | Assert state hash at every snapshot boundary during CI replay tests; round-trip conformance fixture compares hashes at every tick; fuzz-test random seek-and-replay sequences | timetravel-agent | Any CI parity test produces mismatched state hash after replay |
| Browser performance for timeline scrubber degrades with large traces (100K+ events visible) | 3 | 3 | 9 | Virtualized timeline rendering; event dot aggregation at zoomed-out levels; paginated state inspector; lazy loading of snapshot data on-demand | timetravel-agent | Timeline scrubber takes >500 ms to render or respond to scroll on traces with >100K events |
| Debugger integration points change if core scheduler API evolves, breaking trace recording hooks | 3 | 4 | 12 | Trace recorder uses a stable observer trait that the scheduler calls; version the observer API; CI test that verifies trace recording still works after any scheduler change | timetravel-agent | Scheduler PR passes core tests but breaks trace recording smoke test |

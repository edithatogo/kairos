# Time Warp Research Spike

Track 34 remains conservative-first. The local Time Warp spike in
`time_warp_two_lp_spike` models two LPs receiving optimistic events with two
stragglers after later timestamps have already been processed.

Observed deterministic spike result:

| Metric | Value |
|---|---:|
| LP count | 2 |
| Processed events | 8 |
| Straggler events | 2 |
| Rollback events | 2 |
| Fossil-collectable events | 6 |

Findings:

- Optimistic execution needs state snapshots before each speculative event.
- Anti-message cancellation and duplicate suppression must be designed before
  any production Time Warp scheduler can share the Track 34 transport boundary.
- Fossil collection depends on a GVT decision that includes pending in-flight
  event timestamps, matching the conservative GVT rule already used locally.

Recommendation: keep Track 34 on conservative CMB scheduling until rollback
state snapshots, anti-messages, and fossil collection have their own track-level
design and validator coverage.

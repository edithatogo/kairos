# Track 48 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Rollback restores inconsistent ECS state | Corrupt simulation | Generation-aware snapshots and parity tests |
| Anti-message matching is ambiguous | Incorrect cancellation | Stable message IDs and strict envelope tests |
| Fossil collection removes required history | Irrecoverable rollback | Collection only after proven GVT |
| Memory overhead is unbounded | Runtime unusable | Budget metrics and pressure benchmarks |
| Optimistic mode changes conservative behavior | Regression | Separate `time-warp` feature and mode tests |

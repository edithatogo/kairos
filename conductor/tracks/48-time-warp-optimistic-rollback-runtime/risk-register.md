# Track 48 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Rollback restores inconsistent ECS state | Corrupt simulation | Generation-aware snapshots and parity tests |
| Anti-message matching is ambiguous | Incorrect cancellation | Stable message IDs and strict envelope tests |
| Fossil collection removes required history | Irrecoverable rollback | Collection only after proven GVT |
| Model snapshot byte cost is not bounded by event counts | Runtime memory pressure | Finite queue/history limits and snapshot counts; assess model-owned memory separately, no total-byte guarantee |
| Optimistic mode changes conservative behavior | Regression | Separate `time-warp` feature and mode tests |
| Compiler labels drift from actual Cargo compiler | Invalid compatibility/performance claims | Absolute compiler/tool paths and hashes, real Cargo-cache readback, config/wrapper refusal, preserved superseded attempts |
| Local GVT excludes distributed in-flight work | Unsafe distributed fossil collection | Caller-proven local floor only; Track49 must supply participant/in-flight proof |
| Track48 Done requires Track49, which depends on Track48 | Scheduling deadlock or implicit waiver | Preserve InProgress; explicit reviewed phase/interface scheduling decision before Track49 production |

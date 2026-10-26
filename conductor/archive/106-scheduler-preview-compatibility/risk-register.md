# Track 106 risk register

Scoring: likelihood and impact are Low/Medium/High; any High impact is treated
as a release-blocking risk until resolved or explicitly accepted by its owner.

| Risk | Likelihood | Impact | Severity | Mitigation / evidence | State |
|---|---|---|---|---|---|
| Preview accidentally dispatches or advances virtual time | Medium | High | High | Snapshot `now`, counters and pending membership around repeated peeks; then verify `step()` dispatches the same event | Open |
| Cancellation leaves a dead heap head visible | Medium | High | High | Fixture cancels the earliest event and asserts preview returns the next live event | Open |
| Ordering is unintentionally changed | Low | High | High | Only copy the existing heap head; test `(at, priority, sequence)` and compare preview to following dispatch | Open |
| Public API classification is assumed without Track 25 | Medium | High | High | Obtain reviewer-agent assessment of the exact Rust core root and ADR using Track 25 compatibility criteria | Open |
| Conformance fixture is not accepted by Track 12 | Medium | Medium | Medium | Keep fixture language-neutral and obtain reviewer-agent assessment using Track 12 criteria | Open |
| Candidate base differs from CareOps tested source | Medium | High | High | Record exact Kairos commit and CareOps lock/pin readback; no parent update before checks | Open |

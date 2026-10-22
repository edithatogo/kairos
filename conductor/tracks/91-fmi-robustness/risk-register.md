# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | Reject if modes/platforms or maintenance do not meet need; then use bounded zip/quick-xml replacements. If accepted, supersede their redundant direct-adoption scopes through documented owner handoff. |
| Changed semantics hidden by updated goldens | High | Independent reference FMUs, lifecycle errors, get/set/step, platform loading, schema and supported export roundtrip, restore if required and leak/cleanup failures. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

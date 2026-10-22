# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | No global async conversion of core or extra runtime per request; conditional on actual RPC use and net simplification. |
| Changed semantics hidden by updated goldens | High | Real process shutdown/cancellation, slow consumers, saturation, reconnect and bounded resources; deterministic handoff ordering despite different IO timing. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

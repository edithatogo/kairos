# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | No adoption unless net maintained complexity falls and required replay/identity/rollback behaviour survives. Record no-go if mapping recreates most of current storage; retain working ECS. |
| Changed semantics hidden by updated goldens | High | Representative spawn/despawn, sparse/dense query and mutation benchmarks; full ID/reuse limits, relationships, snapshot/rollback, consumer bindings and replay after restore; count all compatibility code. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | Retain spatial hash if it is simpler/faster for qualified workloads; do not add an index without a real query consumer. |
| Changed semantics hidden by updated goldens | High | Independent brute-force neighbour oracle, movement/removal, duplicate IDs, boundaries/nonfinite input and sparse/dense/skew dynamic workloads; update/query RSS/time. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

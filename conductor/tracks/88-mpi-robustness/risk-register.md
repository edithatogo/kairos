# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | No installation for socket-only tests or absent HPC consumer; no CPU/socket fixture promoted to MPI acceptance. |
| Changed semantics hidden by updated goldens | High | Actual MPI implementation and multi-rank runs with independent API evidence, ordering/collectives, failures, deadlock controls and hardware/runtime provenance. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

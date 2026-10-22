# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | Reject if total wrapper and scheduling complexity grows without a demonstrated benefit or accepted workloads regress materially; no claimed real RL capability from synthetic batches. |
| Changed semantics hidden by updated goldens | High | 1/2/4 worker exact traces/results, skewed tasks, small/large ensembles, panic/error propagation, cancellation ownership and thread count/RSS/latency versus baseline. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

# Track 47 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Lookahead rule admits causality violation | Incorrect final state | Add failing tests before runtime implementation |
| GVT stalls under sparse traffic | Deadlock or unbounded memory | Null-message and drain tests with adversarial workloads |
| Parallel runtime changes sequential behavior | Regression for existing users | Keep feature-gated and test sequential workspace |
| Benchmarks overstate speedup | False parity claim | Require raw manifest and Track 46 claim boundary |
| Core scheduler changes leak across ownership | Cross-track conflict | Use handoff for blocked paths |

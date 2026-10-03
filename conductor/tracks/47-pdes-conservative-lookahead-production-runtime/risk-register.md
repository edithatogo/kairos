# Track 47 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Lookahead rule admits causality violation | Incorrect final state | Exclusive channel bounds and emitting-event provenance; production parity and boundary tests |
| GVT stalls under sparse traffic | Deadlock or unbounded memory | Queue-constrained bounds, partial-horizon resume, random and minimum-lookahead 8-LP 10,000-tick runtime tests |
| Parallel runtime changes sequential behavior | Regression for existing users | Keep feature-gated and test sequential workspace |
| Benchmarks overstate speedup | False parity claim | Require raw manifest and Track 46 claim boundary |
| Core scheduler changes leak across ownership | Cross-track conflict | Use handoff for blocked paths |
| Callback failure partially mutates LP state | Unsafe recovery | Join workers, atomically reject outbound batch and permanently poison runtime |
| Same-tick self loop | Non-terminating run | Per-call event budget with typed resumable budget exhaustion |
| Source changes during benchmark capture | False commit provenance | Pushed ref readback, clean source and pre/post source digest equality |
| More LPs than host cores | Misleading performance interpretation | Record real core/thread topology and oversubscription; no superiority claims |

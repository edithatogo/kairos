# Track 68 Risk Register

Severity scale: Low (1), Medium (2), High (3), Critical (4), Release Blocker (5).

| Risk | Likelihood | Impact | Severity | Mitigation |
|---|---|---|---|---|
| In-flight transfer entities lost if recipient crashes or fails to accept | Low (1) | Critical (4) | High (3) | Enforce two-phase handover commit with receipt confirmation |
| Unbounded channel queues cause silent heap memory growth | Medium (2) | High (3) | High (3) | Enforce explicit capacity bounds and upstream backpressure rejection |
| Circular boundary channels cause circular wait deadlocks | Low (1) | High (3) | Medium (2) | Provide deadlock detection and timeout / reneging policies |

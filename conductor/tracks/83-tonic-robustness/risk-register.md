# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | Stop if distributed RPC has no active consumer; do not add HTTP/2 overhead to in-process execution or retain two production transports indefinitely. |
| Changed semantics hidden by updated goldens | High | Independent non-Kairos gRPC peer, two-process failure/reconnect/backpressure and duplicate messages; wire version rejection, transport policy and unchanged PDES ordering. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

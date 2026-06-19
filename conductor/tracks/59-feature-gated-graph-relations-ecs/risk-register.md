# Track 59 Risk Register

| Risk | Impact | Mitigation |
|---|---|---|
| Graph code leaks into default build | Public API drift | Compile-boundary tests without feature enabled. |
| Pointer topology breaks ECS locality | Unsafe or slow graph traversal | Static scan and code review gate. |
| Traversal allocates heavily | Solver performance cliff | Benchmark traversal fixtures before In Review. |

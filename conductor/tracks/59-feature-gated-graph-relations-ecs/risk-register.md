# Track 59 Risk Register

Severity scoring scale: Low/Medium/High/Critical = 1-4. Low = bounded documentation or local workflow inconvenience; Medium = delayed phase closeout or limited user-facing claim risk; High = broken implementation, release-blocking evidence gap, or unsafe public claim; Critical = data race, unsafe memory behavior, credential exposure, or knowingly false release/parity claim.

| Risk | Impact | Mitigation |
|---|---|---|
| Graph code leaks into default build | Public API drift | Compile-boundary tests without feature enabled. |
| Pointer topology breaks ECS locality | Unsafe or slow graph traversal | Static scan and code review gate. |
| Traversal allocates heavily | Solver performance cliff | Benchmark traversal fixtures before In Review. |

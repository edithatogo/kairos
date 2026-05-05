# Risk Register — 01 The Heart: kairo-ecs-core & kairo-ecs-state

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| ECS storage strategy undecided (no ADR) | 2 | 5 | 10 | ADR 0001-ecs-storage-strategy.md documents tradeoffs and final choice | core-agent | No ADR filed before storage implementation begins |
| BinaryHeap ordering not observationally stable across Rust versions | 2 | 5 | 10 | Pin ordering in contract; use lexicographic tie-breaker on stable fields | core-agent | Determinism conformance test fails on any target |
| Seed derivation formula not in contract | 3 | 4 | 12 | Publish seed derivation as part of Track 01 public API contract | core-agent | Any cross-implementation simulation produces different output with same seed |
| Cancellation O(n) worst case | 3 | 4 | 12 | Document worst-case in API docs; benchmark cancellation with regression thresholds | core-agent | Cancellation latency exceeds documented threshold × 2 in benchmark |
| Performance regression without benchmark thresholds | 4 | 3 | 12 | Define performance regression thresholds in CI; compare against baseline in `conductor/benchmarks-reproducibility.md` | core-agent | >20% regression on any tracked benchmark |
| SIMD portability breaks on non-x86 platforms (ARM NEON, WASM) | 3 | 3 | 9 | Gate SIMD behind `#[cfg(target_feature)]`; fall back to scalar path; test on ARM CI runner | core-agent | SIMD path fails on any Tier 1 target platform |
| Formal verification proves a bug that blocks release | 2 | 5 | 10 | Treat verification findings as bugs, not release blockers; fix before next minor | core-agent | Kani/loom/Creusot finds a soundness bug in scheduler invariants |

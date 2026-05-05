# SOTA Gap Analysis

Last updated: 2026-05-05

## Methodology
Compare KairoECS architecture against state-of-the-art in DES, ABM, and multi-language simulation frameworks.

## Current gaps

| Area | KairoECS | SOTA | Gap |
|---|---|---|---|
| Deterministic scheduling | (time, priority, sequence) ordering | Manifold SIMD scheduling | Sequential only; no PDES |
| Multi-language | C ABI + 6 binding targets | Python-only frameworks dominate | Broad surface but none operational yet |
| Arrow integration | Schema contract defined | Columnar-first sim engines | Not implemented |
| Reproducibility | Seed/replay in design | ML reproducibility tooling | No pipeline yet |
| Performance | 1M entity target | GPU-accelerated ABM frameworks | Not benchmarked |
| Community | Governance skeleton | Large established communities | Pre-launch |

## Recommended focus
- Ship v0.1 hero path (Rust core + Python preview) before expanding bindings.
- Benchmark against established DES/ABM frameworks before making performance claims.
- Add PDES research spike after sequential determinism is proven.

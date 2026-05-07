# Risk Register — 05 The Window: kairo-ecs-viz Visualization

Severity scale: Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Viz feature flag leaking into headless build | 3 | 4 | 12 | Strict feature-gate discipline; CI builds headless variant and verifies no viz deps linked | viz-agent | Headless binary links any viz dependency |
| Rendering dependency bloat | 4 | 3 | 12 | Optional feature behind `viz` flag; tree-shake dependencies; track binary size in CI | viz-agent | Binary size exceeds budget by >10% |
| Platform-specific GPU/rendering incompatibility | 4 | 4 | 16 | CI matrix covering target platforms; fallback CPU rasterizer when GPU backend unavailable | viz-agent | Any platform renders blank or crashes |
| Visual output non-reproducibility across runs | 3 | 4 | 12 | Viz snapshot contract (Track 01) defines pixel-exact or tolerance-based reproducibility guarantees | viz-agent | Snapshot test fails on same seed and platform |

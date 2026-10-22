# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | Activate only with accepted geographic route consumer; if boundary types alone suffice, use geo-types and revisit this track explicitly rather than install unused algorithms. |
| Changed semantics hidden by updated goldens | High | Known line/route distances and positions, endpoint/zero-length/multi-segment paths, coordinate-order and unit errors, geographic projection roundtrip and monotonic route progress. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

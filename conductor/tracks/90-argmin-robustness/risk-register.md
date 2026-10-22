# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | No adoption to replace a trivial objective function or absent optimisation loop; identify concrete code deletion/avoided implementation first. |
| Changed semantics hidden by updated goldens | High | Known analytical objectives, convergence/failure/bounds, independent solver reference, reproducible restart and noise/seed handling with justified termination tolerances. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

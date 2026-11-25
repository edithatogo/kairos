# Risk register

| Risk | Impact | Mitigation / evidence owner |
|---|---|---|
| Specific implementation risk | Batching or work stealing can alter causality and stochastic order. Correctness gates precede speed gates, and serial remains available where it wins. | performance-agent + pdes-agent; independent phase oracle before acceptance. |
| Historical evidence reused after source changes | False acceptance or replay drift | Bind source/dependencies/inputs; explicitly reassess affected evidence. |
| Placeholder or unsupported path returns success | Misleading scientific/native result | Negative/mutation controls and explicit unavailable errors. |
| Parallel writers or cross-repository assumptions | Unreviewed source/pin changes | Isolated claims; provider/consumer contract; no private paths. |
| Overstated public scope | Users rely on unevaluated capability | Capability register and release gate; accurate unsupported status. |


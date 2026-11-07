# Risk register

| Risk | Impact | Mitigation / evidence owner |
|---|---|---|
| Specific implementation risk | A schema validator cannot prove a reported experiment occurred. Require actual raw execution evidence and independent review; never turn metadata success into performance, clinical or release acceptance. | conformance-agent + docs-agent + ci-agent; independent phase oracle before acceptance. |
| Historical evidence reused after source changes | False acceptance or replay drift | Bind source/dependencies/inputs; explicitly reassess affected evidence. |
| Placeholder or unsupported path returns success | Misleading scientific/native result | Negative/mutation controls and explicit unavailable errors. |
| Parallel writers or cross-repository assumptions | Unreviewed source/pin changes | Isolated claims; provider/consumer contract; no private paths. |
| Overstated public scope | Users rely on unevaluated capability | Capability register and release gate; accurate unsupported status. |


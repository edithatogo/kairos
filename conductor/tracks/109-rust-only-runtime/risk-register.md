# Risk register

| Risk | Impact | Mitigation / evidence owner |
|---|---|---|
| Specific implementation risk | Breaking callers may currently depend on host traces that ABI1 cannot return; expose only real native operations and document the exact replacement. New native loading expands FFI/lifetime review obligations. | ffi-agent + api-governance-agent; independent phase oracle before acceptance. |
| Historical evidence reused after source changes | False acceptance or replay drift | Bind source/dependencies/inputs; explicitly reassess affected evidence. |
| Placeholder or unsupported path returns success | Misleading scientific/native result | Negative/mutation controls and explicit unavailable errors. |
| Parallel writers or cross-repository assumptions | Unreviewed source/pin changes | Isolated claims; provider/consumer contract; no private paths. |
| Overstated public scope | Users rely on unevaluated capability | Capability register and release gate; accurate unsupported status. |


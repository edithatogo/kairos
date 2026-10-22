# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | No adoption without a required model or if supported operators/targets fail. Replacing a placeholder is new capability, not proven code-reduction or preserved inference parity. |
| Changed semantics hidden by updated goldens | High | Independent reference outputs for real model bytes, unsupported operator/model rejection, shape/type/nonfinite/oversized inputs, timeout/fallback and pinned model/version hashes. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

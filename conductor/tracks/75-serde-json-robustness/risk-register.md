# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | No direct dependency where an existing accepted upstream API already serializes the same object; no second JSON representation. |
| Changed semantics hidden by updated goldens | High | Independent parser reads escaped Unicode/control characters and exact integer/string representations; malformed, duplicate-key and oversized inputs reject as specified. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

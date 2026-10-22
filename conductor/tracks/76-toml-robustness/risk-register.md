# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | Stop if changed parsing silently changes effective inputs; no compatibility mini-parser without an explicit need. |
| Changed semantics hidden by updated goldens | High | Sections, comments, quotes/escapes, arrays, duplicate keys, malformed tokens, missing fields, out-of-range values and existing fixture migration; independent valid TOML fixtures. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

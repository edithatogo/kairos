# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | If rust-fmi schema APIs already cover the consumer, retire this direct-adoption path rather than keep a second parser. |
| Changed semantics hidden by updated goldens | High | Namespace/attribute ordering, escaped content, malformed/truncated XML, false root substrings, unsupported FMI versions and independent consumer parsing. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

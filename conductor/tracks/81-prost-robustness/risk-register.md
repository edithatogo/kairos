# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | No codec migration without an approved interoperable RPC consumer; generated code volume is not maintained-code savings. |
| Changed semantics hidden by updated goldens | High | Independent protobuf peer roundtrip; malformed/oversized/unknown fields and versions; full ticks/IDs, optionality and canonical semantic equality. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

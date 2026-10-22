# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | Retain code if serialization cannot preserve identity/schema semantics; do not derive unused DTOs. |
| Changed semantics hidden by updated goldens | High | Roundtrip full-range IDs/ticks, optional fields and invalid versions; unknown/duplicate fields follow explicit reviewed policy; bounded input and no silent defaulting. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |

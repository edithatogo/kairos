# Risk register

| Risk | Mitigation and stop condition |
|---|---|
| Library/version/MSRV/ABI mismatch | Qualify exact feature graph and FFI conversion before migration; stop on unsupported target |
| Fake-format self-roundtrip | Independent PyArrow reader, compression metadata and actual value assertions |
| Tick overflow/borrow/precision | Full-u128 limb fixtures, checked subtraction and numeric ordering |
| Joins duplicate/drop entities | Uniqueness/cardinality/orphan checks and conservation oracles |
| Wrong scientific estimand | Freeze denominator/weight/censor definitions and analytical expectations |
| Unbounded streaming/hash joins | Skew/high-cardinality memory tests, bounded trace capture and measured budgets |
| Trace loss/duplicate after crash | Part identity and checkpoint watermark; interruption tests across publication boundaries |
| FFI lifetime/data race | Immutable owned batches; multi-batch/release/engine-destruction tests |
| Clean break strands data | New schema IDs; preserve historic files; reject unsupported formats; no implicit conversion |
| Dependency/build cost replaces code debt | Report maintained-code reduction and build/binary/RSS costs; no wrapper framework |
| Incomplete cross-repository coordination | Accepted immutable upstream receipt before parent pin; do not advance on status alone |

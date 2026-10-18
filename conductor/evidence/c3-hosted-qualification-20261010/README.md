# C3 hosted synthetic qualification — 10 October 2026

Maturity: experimental/private. Qualified source `faeb8e97d3021080c2dd60f0e2f065607e7666e7`
on [draft PR244](https://github.com/edithatogo/kairos/pull/244).
[Native owner run38047961900](https://github.com/edithatogo/kairos/actions/runs/38047961900)
completed successfully. All 47 successful checks and two conditional skips are
retained in [checks.json](checks.json). Q5.2 canonical branch-only regression and
Codecov OIDC upload were skipped; neither is counted as passed.

Both native hosts used Rust/Cargo 1.99.0: Ubuntu x86_64 and macOS aarch64. The CI
checkout merge `af2d53f7df7474a2efe9fc89b1bb880e867f3704` has tree
`8919c82573c75e549916d2317745eff52b315d6c`, identical to the reviewed PR head.
Each host ran 26 passing shadow tests; the only ignored entry was the child
entrypoint explicitly invoked by its passing parent. All five child invocations
passed. The native physical IPC/Parquet test passed on both hosts.

Independent reviewers `c3_semantics_preparation` (Ubuntu) and
`c3_recovery_preparation` (macOS) matched the source hashes, toolchain, source tree,
test counts and all recovery outputs. Baseline and restored result JSONs and both
checkpoint images are byte-identical across hosted architectures and local runs.
This is an observed synthetic result, not a general architecture portability claim.

[Readback](readback.json), [run](run.json), [PR binding](pr.json),
[archive manifest](archive-manifest.json), and [native artifacts](native-artifacts.tar.gz)
retain the exact evidence. Earlier local gates and review are in
[local qualification](../c3-local-qualification-20261010/README.md).

C3 synthetic implementation qualifies at this source. This documentation successor
requires its own clean phase/DAG and hosted checks before the parent pin advances.
Historical whole-owner Done labels remain their original scope. C5, public API,
clinical/empirical validation, ED MVP and release remain separate.

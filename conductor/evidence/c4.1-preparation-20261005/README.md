# C4.1 experimental fixture preparation — 5 October 2026

Accepted fixture/test preparation at source
`6a17578a16d4bb13969961dfd116b3afaf7c8c6c`. This is no production metric
implementation, Arrow sidecar qualification, C-04 pass or clinical acceptance.

## Executed evidence

- 42 analytic and edge fixtures; 28 numeric cases independently match exact
  inverse-CDF W1 and direct normalized-CDF KS calculations.
- 23 separately pinned SciPy W1/unweighted-KS crosschecks; weighted KS is
  descriptive exact reference only. Wheel SHA-256 matches official PyPI metadata.
- 8 native tests pass on Rust 1.99 and the 1.88 floor; 10 Python tests pass.
  Full calibration compatibility at the unchanged native source passed 72 tests
  with 3 explicit ignores on each toolchain. Fresh final fixture tests are separate.
- Strict Clippy and workspace Rustfmt pass; existing Ruff 0.16.6 lint/format pass.
- Opt-in C4.2 gate fails for both absent actual output and a labeled comparator
  mock. Those expected red controls are retained, not ordinary test passes.

[Acceptance](acceptance.json), [independent review](independent-review.json),
[source boundary](source-boundary.json) and [review report](review-report.md)
state the scope and exact hashes. [Qualification archive](qualification.tar.gz)
retains command/cwd/head/toolchain/input/log hashes, failures, worker packets,
scoped final receipts and source snapshots. [Inventory](artifact-inventory.json)
and [build receipt](archive-build-receipt.json) prove all 137 member bytes by
readback. [Two rebuilds](archive-reproduction.json) produce identical archive
SHA-256; [checksum](SHA256SUMS) is retained. Caches/wheels/venv/private leases are
excluded. The optional Python wheel pin was reproduced only on named macOS ARM.

## Reviewed corrections and limits

Rational parsing, source/metric count separation, null composition, empty
statuses, matched unit scaling and mock rejection were corrected during review.
The original frozen contract and three addenda remain individually hash-bound.
One ignored receipt-path reservation deviation is [retained](coordination-deviation.json);
final gates were repeated under reserved paths. This does not qualify unattended
agents or authenticate a served model. Rust 1.88 retains three pre-existing
C1 adapter lint-expectation warnings; strict current-toolchain Clippy passes.

C4.2 must bind an actual source/API producer and independently execute these
fixtures; JSON provenance alone cannot authenticate execution. C4.3, C4.4/C-04,
clinical validation and release remain open. No upstream phase/registry status
is advanced by this leaf. Parent acceptance and exact-head hosted successor
checks are recorded separately after publication.

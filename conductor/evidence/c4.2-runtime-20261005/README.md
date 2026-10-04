# C4.2 private runtime qualification — 5 October 2026

Runtime source `59d7dbb0c004654e3da90951d42e2b983ef23993` implements deterministic sorted-CDF W1/KS D, exact signed u128 paired residuals, descriptive bias/MAE/RMSE, externally fixed groups/source-clock windows, coverage and validity statuses. Public API review, C4.3 Arrow sidecars, C4.4/C-04, clinical and release acceptance remain open.

## Executed evidence

- All 42 frozen C4.1 cases pass against an actual source-bound report on Rust 1.99 and 1.88; the unchanged independent Python exact Fraction comparator also passes.
- Full calibration compatibility: 119 tests pass on each toolchain; four named opt-in tests remain ignored by default. The runtime candidate gate and synthetic timing are explicitly executed separately.
- 13 distance, 23 residual, four cohort and seven producer tests pass. Strict all-target calibration Clippy and workspace Rustfmt pass. Existing fixture/phase/DAG integrity checks pass.
- Wrong commit, wrong toolchain and mutated actual output each fail with expected exit101. Reports bind observed Git HEAD, committed compiled producer/kernel bytes and observed rustc; metadata alone is not runtime proof.
- One debug synthetic check, 100000 points per side, W1=1 and D=1/100000, took2256ms on the named Mac. This is not a release benchmark or speedup claim. Pairing now uses an ordered map rather than quadratic scans.

[Acceptance](acceptance.json), [integrated review](integrated-review.json), [actual report](actual-runtime-report.json), [independent readback](independent-actual-readback.json), [source boundary](source-boundary.json), and [coordination deviations](coordination-deviations.json) retain exact scope. Command receipts/logs record cwd/head/argv/env/toolchain/input hashes/exit status. [Archive](qualification.tar.gz), [inventory](artifact-inventory.json), [build receipt](archive-build-receipt.json) and [checksum](SHA256SUMS) retain and read back every member. Build caches, private claims/tokens and patient data are excluded; all inputs are synthetic.

## Limits and reviewed fixes

Exact rational arithmetic is bounded to checked u128 intermediates and returns Invalid on overflow. Individual residual signs/magnitudes remain exact; summaries are deterministic compensated binary64. The approximation flag identifies magnitude conversion loss, not every division or square-root rounding. Counts for excluded/censored/missing/failed/infeasible rows overlap independently; they cannot be summed as a denominator. Invalid inputs preserve all raw rows and groups.

Review corrected weight-order dependence, precision metadata, invalid/null counts, commit/source/toolchain binding, whole-batch residual retention, pair group/time coherence and wrong-side ticks. A reported unknown-group panic was retracted after complete source readback; an explicit regression and fault-injection control are retained. Historical lease/packet/output-path deviations remain disclosed; this is no certification of unattended harness compliance. Rust1.88 retains pre-existing C1 lint-expectation warnings. Hosted successor acceptance and parent publication are recorded separately after delivery; no upstream phase or ledger is advanced by this leaf.

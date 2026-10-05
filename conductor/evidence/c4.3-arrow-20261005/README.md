# C4.3 actual Arrow sidecars — 5 October 2026

Source `543a5dd4c6b3b3b2b890ee258fa7e77cea86505f` implements private calibration_residual.v1 and calibration_metric.v1 codecs and the actual C4.2 runtime adapter. Residual and metric sidecars use real Arrow IPC file/stream and Parquet IO. Run/event joins and full raw residual/metric provenance remain separate from the unchanged event_log.v1 format.

## Executed local qualification

- Rust1.99 both-feature calibration package:140 tests pass; four named opt-in gates remain ignored by default. Strict all-target Clippy and workspace Rustfmt pass. Rust1.88 executes all21 C4.3 tests; default calibration compatibility also passes in the retained source-bound run.
- None, IPC, Parquet and both feature configurations were executed on both toolchains. Arrow dependencies remain optional, default features empty, and existing dependency versions unchanged.
- The actual fixture reconciles15 raw residual rows to8 full-key pair records,3 contributing pairs and9 unmatched raw instances. Seven metric rows include paired bias/MAE/RMSE and fixed W1/KS cohorts, including typed empty/null outputs. Counts for censoring, missing, failure and exclusions overlap and cannot be summed as a denominator.
- Pinned Python3.14.8/PyArrow25.0.1 independently reads actual producer bytes on both toolchains, across framing sizes1,2,64. Exact schemas/types/nulls, full-width little-endian u128 values, logical records, raw hashes, run/event joins and legacy smoke bytes reconcile. Fifteen actual-manifest controls reject mutations and missing/empty required evidence. Four physical mutation controls reject wrong type, corrupted u128, required null and wrong metadata.
- C0 logical schema, event schema/library and C4.2 metric/cohort/residual kernels are byte-for-byte unchanged. Review fixed censor/missing lineage when prediction failure determines outcome. A reported mixed-clock reader issue was withdrawn after the unchanged kernel rejected contradictory pair clocks; an explicit rejection regression retains both raw sides.

[Acceptance](acceptance.json), [source binding](source-binding.json), [review](integrated-review.json), [unchanged contracts](unchanged-contracts.json), and [coordination deviations](coordination-deviations.json) retain scope and actual command evidence. [Archive](qualification.tar.gz), [inventory](artifact-inventory.json), [build receipt](archive-build-receipt.json) and [checksums](SHA256SUMS) retain actual producer archives/records, packets, successful and failed receipts, and independent worker readback. All data are synthetic. Build caches, environments and private leases/tokens are excluded. Independent archive readback is retained separately when complete.

## Delivery boundary

The final native receipts bind the exact executed source commits and compiled Rust source hashes; final reader/CI-only successors do not change those compiled bytes. Hosted native owner success on Linux x86_64/macOS ARM at the published successor, and parent pin acceptance, are recorded separately. This leaf does not close C4.4/C-04, approve a public calibration API, release or clinical use. The adapter currently supports empirical_equal.v1 nanosecond/scale1 export; weighted/rescaled export needs a separate owner contract. No stable dependency/support gate is waived.

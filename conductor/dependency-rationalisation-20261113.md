# Rust dependency rationalisation — kairos (2026-11-13)

**Status:** scoped Conductor adoption contract; code substitutions not yet implemented or certified by this file.

## Repository-specific decisions

- Reconcile existing `dependency-frontier-20261029`, simulation reproducibility, compatibility and CareOps-Sim upstream contracts; preserve pinned Polars/hashbrown patch and consumer pins.
- **Verified:** multi-crate ECS/DES/ABM, Arrow/data, GPU, networking and bindings architecture; root MSRV 1.99.
- **Evaluate:** Rayon only for independent deterministic partitions and stable reduction order, with worker-count equality; Crossbeam only for measured synchronous channel need.
- **Evaluate:** Polars through existing qualified upstream pin; retain Arrow/Parquet schema and source-provenance requirements.
- **Extend:** Proptest for event invariants, Criterion for CPU kernels and scenario benchmarks for distributed/GPU work, small redacted Insta output snapshots.
- **Retain:** custom simulation scheduler, RNG streams and clock. No Tokio in CPU core merely for standardisation; PyO3 only within bindings crate and Syn/Quote only for procedural macros.
- **Use tracing selectively:** bounded dispatch/operation telemetry with observer-effect benchmarks.

## Delivery and acceptance contract
- Reconcile existing Conductor tracks and ownership before editing code; do not duplicate completed tasks.
- Write a failing behavioural/regression test first for each runtime change; preserve APIs, schema versions, stdout/stderr, exit codes, error codes and MSRV or explicitly version breaking changes.
- Run the existing repo gates: targeted tests, full workspace tests, fmt, clippy and contract/interop checks. Record actual passing CI SHA; this document is planning, not validation.
- Inventory duplicate crate versions/features with `cargo tree -d` and `cargo tree -e features`; review unused dependencies, `cargo audit`, `cargo deny`, licence, MSRV, SBOM and release implications where applicable.
- Benchmark performance proposals with fixed fixtures and baseline/candidate receipts, including correctness, memory, CPU and determinism.
- Decision for each candidate must be recorded as adopt, retain, reject or defer with evidence, risks and rollback.
- Do not expose identifiable, secret, copyrighted or otherwise restricted raw data in tracing, snapshots, benchmarks or CI artifacts.

# C1.4 integrated qualification — 2026-10-05

Runtime source: `7f72b7d9a9b6ae62e481a59b2cc04ce37da0a4a5`. Integrated source: `87d591c484bc937f7536a0e700b1400968b9549a`. Final governance successor acceptance is recorded by the parent after fresh gates.

The retained `qualification.tar.gz` contains 1,969 members (5,509,128 bytes), SHA-256 `1955ee5a36dc190cf83e7af308c0231a3aed92a4f0e17b0604b108b6ca013a03`. Every member is hashed in `artifact-inventory.json`; construction and full verification are recorded in `archive-build-receipt.json`. Build caches and virtual environments are excluded.

Accepted readback candidates are `independent/readback-accepted-candidate.json`, `independent/reconciled-counts-accepted-candidate.json` and `independent/execution-receipt-accepted-candidate.json`, using `independent/independent_readback_v2.py`. These record complete executed argv, source/worker heads, toolchain, UTC times, exit codes and input/output hashes. Earlier failed setup, rejected readback and superseded receipts remain historical attempts in the archive. Final independent disposition is retained separately.

Fresh proof includes 59 calibration tests, 24 physical adapter invocations, 648 ordered C-01 file comparisons, 108 actual transport runs plus 3,780 exact byte aliases, 51 actual mapper requests, fresh typed Rust output and 666 independent IPC/Parquet file reads. Eleven negative checks cover physical schema/units/nulls plus inventory corruption, expected-source mismatch and an actual stale checkout HEAD. Logical UTC and map field counts are checked after decoding physical representations.

C-01 distinct profiles reconcile 17 source rows to 23 candidates: 18 mapper accepted and 5 excluded; accepted records split into 15 valid and 3 quarantined. Six outcomes split into 3 observed and 3 right censored. The separate C1.1 fixture collection has 51 requests, 56 source rows and 59 candidates: 31 accepted, 17 excluded, 6 failed and 5 unresolved. Replicas and alias points add no distinct records. These are synthetic fixture populations.

`runtime/integrated-source-preservation.json` proves 90 C1 runtime paths unchanged from tested runtime to integrated source. Accepted Q5.2 source is retained byte-for-byte except additive changelog text. Integrated hosted CI has 48 successes and two expected conditional skips (Codecov upload and branch-specific Q5 canonical comparison); historical Q5 canonical evidence remains at its accepted source. Fresh governance-successor checks are retained in the parent to avoid a self-referential evidence commit.

No dependency, public API, schema, golden, security gate or performance threshold was relaxed. C2, public API review, clinical validation, release and Q5.3/Q5.4 remain independent. Historical Track 04/21 closure is supplemented, not widened.

The `superseded-logical-null-*` archive/inventory/receipt preserve the prior rejected verifier report. They are not acceptance evidence. The final verifier restores generic logical null/empty traversal before schema-aware absent-field checks.

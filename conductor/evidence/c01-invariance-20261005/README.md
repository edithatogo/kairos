# C-01 synthetic invariance acceptance — 5 October 2026

Qualified source: `edcc4b353485141b69521e616d9a05ef50ff3042`.
The acceptance receipt binds exact source hashes, archive checksum and inventory.

- Three long/wide/quarantine profiles; four canonical populations compared by
  exact NDJSON bytes, ordered records, counts, byte lengths and SHA-256.
- Independent IPC batches 1/2, Parquet row groups 1/3 and forward/reverse orders;
  24 actual Rust adapter runs, 648 ordered schema/metadata/payload checks and
  216 actual reader-derived input bundles across three formats/writer limits.
- 3,888 normalization points: 108 actual executions and 3,780 explicitly checked
  exact-input aliases. Six representative actual output sets are retained.
- Chunk rows 1/2/64, sort rows 1/2/64 and byte caps 4,096/1,048,576;
  observed row spills, byte-only spills and single-run controls all pass.
- Schema-valid raw marker and physical payload mutations are detected; all
  three actual baselines pass independent C0 schema/accounting validation.

`qualification.tar.gz` retains source captures, all physical files and boundary
receipts, reader bundles, all point/execution ledgers, representative outputs,
command/cwd/tool/input/output hashes, logs, negative failures and independent
review. `artifact-inventory.json` permits exact member verification. No caches,
credentials, operational leases or real patient data are included.

Original capture configs lacked an explicit policy block. Original configs are
retained; the documented policy supplement proves original source/template and
all four population bytes unchanged. Failed interpreter/path attempts are kept.

C-01 passes for these synthetic fixtures and declared dimensions. Full C1.4,
clinical validity, hostile-decoder memory qualification, stable API and release
readiness remain separate. Existing broader upstream CI failures are not waived.

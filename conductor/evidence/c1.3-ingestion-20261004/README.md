# C1.3 ingestion development acceptance — 4 October 2026

Private normalization, external sorting, partial-order and explicit half-open
occupancy validation now join in one fail-closed pipeline. Raw source NDJSON
is parsed directly from the fingerprinted files, with duplicate member names
(including nested escaped names) rejected. Consumed byte digests match before
and after fingerprints. Chunks, spills, merge fan-in, identities, cases and
validation comparison work have explicit bounds. Complete-case quarantine,
exclusions, outcomes and manifest counts/hashes are retained. No silent repair.

Both Rust 1.88 and 1.99 calibration suites pass 57 tests. The optional transport
fixture is ignored in ordinary runs and was executed explicitly twice against
actual transported reader output. Actual PyArrow input layouts (batch/group
1/1 and 2/3 reversed) traverse Rust IPC file, IPC stream and Parquet readers
and writers. Python independently verifies all 54 output schemas, metadata
and logical populations; reconstructed source rows re-enter the actual pipeline
and preserve population hashes. The encoder emits UNCOMPRESSED Parquet directly
for this feature-minimal build. Required build evidence, input hashes and
population conservation are checked before the completion manifest is published.

`qualification.tar.gz` retains executed command/cwd/tool/source/input/output
evidence, actual synthetic physical archives, logs and failed attempts.
`acceptance.json` binds the unchanged production Rust source at 943ed42 to the final encoder.
A subsequent test-only adapter change adds schema-mutation rejection assertions;
matching Rust 1.99 Clippy with warnings denied and both actual physical layouts
pass again. `adapter-ci-qualification.tar.gz` retains these commands, source
hashes, outputs and failed attempts. No production ingestion source changed.
Independent C0 schema/population checks reconcile 7 source candidates to
6 events, 1 exclusion and 2 outcomes. No planned or ordinary ignored test is
reported as passed.

This accepts C1.3 for development. C1.4, real-feed qualification, stable public
API, clinical validation and release readiness remain open. Hosted native-owner
acceptance is recorded separately by the parent at its exact final pin.

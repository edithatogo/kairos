# C3 local synthetic qualification — 10 October 2026

Maturity: experimental/private. Runtime source `6f2c4e305e8b885b67f534035e5a72ac5673a094`;
Rust 1.99-only hosted gate `4e7d6075861d42a8f480ca0feb04414fa29d9bd1`.
C2 baseline remains `63a72aa563258777686ee3d2e25794628daae4ec`.
This records local qualification; hosted C3 acceptance and parent pin remain pending.

Three supervised GPT-6-luna workers implemented disjoint ledger/pool/report,
native model, runner, recovery and metadata leaves. The coordinator owned shared
contracts, wiring, cross-component fixtures and acceptance. Independent worker
reviews found and corrected failed-dispatch accounting, future visibility,
Submitted-target restore, nonzero-anchor initialization and aggregate metadata
allocation defects. No new public API or physical schema was introduced.

## Executed gates

All native commands used Rust 1.99.0 on aarch64-apple-darwin, with locked Cargo dependencies.
Exact argv, cwd, profiles, statuses and log hashes are retained in the archive.

| Gate | Result |
| --- | --- |
| Full calibration, flow + test-support + IPC + Parquet | 351 passed, 5 named ignores |
| Release shadow/native/recovery | 26 passed, child entrypoint ignored by top-level collection and explicitly invoked by parent |
| Default calibration | 255 passed, 4 pre-existing named ignores |
| DES/ABM/CLI owner regressions | 348 passed, 1 named ignore |
| Strict all-target calibration Clippy | Passed |
| Rustfmt and diff checks | Passed after formatting two assertions |
| Conductor phase/DAG, VVUQ, conformance, evidence boundaries | Passed |
| Track 21–27 aggregate docs/smoke | Passed after locked website dependency installation |
| Hosted workflow actionlint | Passed; hosted runtime run pending |

The full suite's other four ignores are the existing actual-source transport,
transported-row, timing and C4 candidate-report gates. None is counted as passed.
The native test was re-executed at the committed source after formatting.
Seven independent process sets across repeated debug/release runs had identical
baseline/restored result bytes and identical transit/work checkpoint bytes.

## Evidence

- [Structured result and boundaries](result.json)
- [Conductor review and manual readback](review.md)
- [Source hashes](source-hashes.json)
- [Archive manifest](archive-manifest.json) and [retained local evidence](local-evidence.tar.gz)

Archive members were independently read back against every manifest SHA-256.
It contains earlier failures as well as the named successful successors. The
initial C03 receipt remains a historical regression record, not current acceptance.
Full regression tests also emitted their existing ignored C2 artifact directories;
those generated paths were omitted from the initial C3 reservation. This scope
accounting defect is recorded in the result and must be addressed in future gate
packets. No tracked source outside the writer scope changed.

Local proof does not establish Linux/macOS hosted qualification, C5 scheduling,
public API readiness, clinical validity, ED delivery or release acceptance.
Metadata limits bound estimated individual inventories, not process-wide peak RSS.

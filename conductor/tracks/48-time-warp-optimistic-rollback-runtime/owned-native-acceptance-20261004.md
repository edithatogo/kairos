# Owned native acceptance — 4 October 2026

Track 48 remains In Progress. Independent review accepts the native handler-owned execution, cancellation, replay and retirement leaf at source `9bf478ec653e607b34c90485f427e3d26eacb5e6` under the frozen primary/API contracts. Full distributed acceptance is pending.

The implementation preserves source authority and actual input incarnations, exact output obligations, latest-successor and ancestor barriers, reserved local replacement slots, atomic participant cuts, strict fossil floors and bounded root identities. Snapshot/restore/model/destructor callbacks execute outside peer gates; fault compensation and committed cleanup retain their distinct outcomes. Eighteen independent execution cases cover rollback/state/RNG parity, failure and destructor paths, P/N1/N2 proof orders, blocked replacement and public pressure reports, self delivery, lost admission acknowledgments, strict-cut retries, cascades and actual reduced fanout.

Integration exposed a real public report defect: a blocked local replacement occupied capacity but was missing from pending-event and pending-positive totals. The held-out reporting oracle stayed intact; source9bf478e adds the existing reservation count to both totals. Qualified fixture compiler/helper corrections were separately reviewed; original author candidate and superseded RED artifacts remain retained locally.

## Executed, independently reviewed evidence

Canonical exact-byte copies are in `benches/pdes/evidence/track48-owned-9bf478e/`; manifest SHA entries bind each receipt, raw log and compiler-cache copy. Copies became read-only during packaging; original cache snapshots were mode0644, not historically OS immutable.

| Actual check | Result |
| --- | --- |
| Matching absolute Cargo/rustc/rustdoc1.98.1 locked pdes,time-warp suite |166 passed,0 failed/ignored/warnings |
| Matching absolute Cargo/rustc/rustdoc1.76.0 same suite |166 passed,0 failed/ignored/warnings |
| Explicit1.98.1 +matching LLVM22.1.8 `just ci` |530 passed,0 skipped; core512/55392.59%; fmt,strict Clippy,rustdoc,deny,audit passed |
| Locked all-feature workspace doctests |Exit0;25 crate groups,zero runnable doctests |
| Locked benchmark compilation |Exit0 |
| Collector negative/validation suite |12 passed |
| Strict sparse/dense benchmark collector |Exit0;source/compiler-bound local parity smoke |
| Local HPC evidence manifest |Exit0;shape/claim-boundary validation only |
| Conductor phase and clean Git closeout |Both exit0 at clean native source9bf478e |

Receipts record actual command,cwd,HEAD,tool paths/versions/binary hashes, frozen input hashes, before/after source hashes, exits and output/cache hashes. The final runtime source was unchanged through both compiler runs and fullCI. Standalone fmt/Clippy preceding the final commit bind the same accepted source bytes. Startup failures for an unavailable optional MSRV formatter and an incorrect Cargo plugin version query are retained honestly; neither launched the claimed check or proves a pass. No1.76 formatting pass is claimed.

The benchmark is a tiny single-host lightweight-handler run-call smoke. It excludes setup/extraction/validation/fossil costs and does not measure owned-handler performance, simultaneous CPU execution or distributed rollback. No general speedup is asserted.

## Remaining delivery and acceptance gates

Native implementation PR, exact-head hosted checks and normal merge remain pending at this recording. Global Conductor status/phase files are owned by parallel Q4 work and have not been modified. Parent pin updates require separate ownership after merge.

The external-accounting interface document is a reviewed proposal, not frozen production APIs or execution authority. Native proof capabilities remain process-local. Real MPI2/4-rank and gRPC2-OS-process rollback/cancellation, failure/recovery, migration and distributed GVT evidence are still required before Track48 Done.

The Track29 conditional Track49 entry ADR expires when its bound interface/local semantics change. This implementation triggers that condition. Renew the specific human phase-entry disposition after accepted native PR/checks/merge and a reviewed, bound distributed/PDES/security packet; do not reuse the earlier approval, EXC199, or a hash refresh. Raw dependency gates and full-track acceptance remain unchanged.

## Hosted compatibility fixes — source8b56c94

PR207 at initial sourcee27ab75 exposed two failures absent from all-feature pinned local lanes: newer stable Rust deprecated AtomicU64fetch_update, and one construction heldout lacked its crate-level time-warp feature guard in default-feature workspace builds. Commits53a9ac7 and8b56c94 replace both ID allocations with equivalent Acquire/AcqRel checked weak-CAS loops and add only the missing feature guard; all featureenabled oracles are preserved. Independent review confirms unique prior IDs, no mutation on overflow, original atomic ordering and safe retry behavior.

Source8b56c94 passes Rust1.99 default-feature workspace305 tests, plus166 featureenabled PDES tests on each explicitly bound1.98.1/1.76.0 compiler. Matching1.98.1/LLVM fullCI rerun passes530/530,zero skipped,core92.59%,fmt,strictClippy,rustdoc,deny,audit. Rust1.99 strict all-feature workspaceClippy also passes. Source-bound receipts/rawlogs/caches and original hosted failures are separately retained in benches/pdes/evidence/track48-hosted-fixes-8b56c94/. Earlier9bf evidence remains honestly source-bound and historical. Current hosted rechecks and normal merge are still pending at recording; distributed and renewed Track49 phase-entry gates are unchanged.

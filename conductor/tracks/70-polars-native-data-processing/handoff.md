# Track 70 handoff: Polars-native data processing and standard interchange

## Polars 0.55.2 qualification — 29 October 2026

The user explicitly requested upgrade and qualification against Rust Polars
0.55.2. Integration chat `01a18344-cfb8-7711-8c38-fef1052b9fcf` owns the
Arrow manifest/source, Cargo.lock and this track's qualification records in
`/Volumes/PortableSSD/codex-worktrees/kairos-arrow-publication-repair-20261029`.
Both direct Polars dependencies now select exactly 0.55.2. A fresh isolated
build target is `/Volumes/PortableSSD/codex-builds/kairos-polars0552-20261029`.
Next: compile/API adaptation, focused Arrow/IPC/publication tests, package and
dependency policy qualification, then platform checks. No native objects,
authentication, downstream pin or upstream publication is in scope. Earlier
accepted-version evidence remains historical; this candidate is not qualified
merely because dependency resolution succeeds.

The adapted library compiles on macOS ARM64/Rust 1.99.1. All 65 selected tests
pass: 21 library, 7 native table/IPC, 19 durability, 13 protected publication and
5 tempfile tests. DataFrame construction now supplies the explicit row count;
column access follows the renamed Polars API. Cargo-deny advisory/licence/source
checks pass, but cargo-audit fails with five vulnerabilities: the three existing
zstd findings plus two quick-xml 0.39.4 findings introduced through object_store
0.13.2 (which requires quick-xml 0.39; fixes begin at 0.41). No security waiver,
upstream patch promotion or complete qualification claim is made. Raw local logs
are `/tmp/kairos-polars0552-{check,tests,audit,deny,clippy,package}.log`; build
cache is the isolated target above. Hosted platform and extracted-package proof
remain required, and the candidate must remain draft while the audit fails.

## Protected-root implementation checkpoint — 29 October 2026

The existing integration chat `01a18344-cfb8-7711-8c38-fef1052b9fcf` continues in
the same Arrow repair worktree and branch. The user explicitly selected enforced
protected output roots. Scope now includes both publication implementations,
`RunOutput` resume/read/complete, and the isolated native permission helper
`crates/kairo-ecs-fs-guard`. No tenant objects or authentication are involved.
Earlier assignments below remain history. Next operation is platform and package
qualification of this frozen source, followed by hosted checks when safe.

Local commit `f50e57a` introduced the guard and the first publisher integration.
Commit `df964be` applies the same boundary to `RunOutput`, opens runs
relative to held roots, uses no-clobber staged publication, holds reader handles,
and preserves resume after abandoned staging files. The ADR in this track
records the approved behavior change, accepted filesystem profiles and limits.

A read-only independent review identified Windows file-only inherited ACL grants,
ambient reacquisition in `RunOutput`, and deterministic staging-name collisions
on resume. The candidate addresses all three; Windows runtime verification is
still required. macOS aarch64 Rust 1.99.1 focused verification passed 7 native
table/IPC tests, 19 durability tests, 13 durable-publication tests and 2 native
guard tests. Focused Clippy with warnings denied passed. These are local results,
not hosted acceptance or a declaration that the security finding is closed.

Draft PR `https://github.com/edithatogo/kairos/pull/338` carries the Arrow lane.
Commit `94a8834` adds the Linux/macOS/Windows runtime qualification workflow.
Read-only integration preflight against main `534c9bd` found clean source and
target worktrees, no dirty-path overlaps, no staged deletions and no warnings.
The first package-inventory validation identified the new helper missing from
the release manifest; its registration is included in this follow-up. All three
Rust source archives (types, guard, Arrow) were produced without verification;
that establishes archive creation, not full consumer or security qualification.

Hosted run `40808879819` passed the macOS platform job. Windows passed its native
guard and Arrow runtime/unit steps, then failed a Clippy test-module placement
rule. Linux identified that capability `open_dir_nofollow` yields lookup-only
`O_PATH` descriptors, which cannot be synced. The follow-up opens real readable
Unix directory handles without following symlinks, while preserving Windows'
directory-handle behavior, and adds a directory-sync regression test. Test modules
are moved after production items for the Windows lint. Local macOS focused
durability/publication tests and the four path/sync tests pass; focused Clippy
passes. Hosted reruns are still required for these corrections.

The Linux VM's default umask also exposed group-writable test fixture roots.
The policy correctly rejected them; fixtures and the benchmark now request
private roots explicitly without changing a process-global umask. On macOS, all
58 selected Arrow unit/durability/publication/tempfile tests pass under a child
shell umask of `0002`, and focused Clippy passes. The Linux run uses one Cargo
build job after parallel test linking hit the VM memory limit; first-failure
logs are retained rather than treated as a transient test retry.

Separate native checks demonstrate that a Linux second identity cannot write
through a masked ACL, while an effective write grant is rejected by the guard.
A macOS ACL-denied security query returns a guard error rather than acceptance
of an apparently absent ACL. These are synthetic local fixtures, with retained
source and raw logs in the Codex Security standalone artifact collection.

An isolated Linux aarch64 qualification VM now runs at
`LIMA_HOME=/Volumes/PortableSSD/kairos-lima`, instance `kairos-guard`. Its fresh
Ubuntu minimal image matched the official SHA256
`6e971c7935c62c8ea7796b085e4ea038e2301a067f8c853e154133f8df192c85`.
Its tools and task directory are separate from the stopped CareOps VM. The
earlier attempt to start that VM failed downloading container tooling into the
full system cache; no guest operation was started there.

The zstd package blocker remains: workspace-only patches do not propagate into
the packaged consumer, and a feature-split experiment still retained vulnerable
optional versions in the regenerated lockfile. No audit ignore, exception or
release/merge claim is introduced. Full security, package, platform and PR gates
must pass before merge and pruning.

## Publication repair assignment — 29 October 2026

The user assigned integration chat `01a18344-cfb8-7711-8c38-fef1052b9fcf` the
bounded Arrow publication repair. Its isolated standard Git worktree is
`/Volumes/PortableSSD/codex-worktrees/kairos-arrow-publication-repair-20261029`.
The registry names Arrow, FFI and interoperability roles; no live Arrow-path
lease or Arrow-owner worktree was identified. This assignment covers the
durable Polars manifest, writer/reader, shared atomic-write sync and focused
tests. FFI, bindings, native objects, dependency promotion and CareOps pin
ownership remain separate. Next: repair and focused tests, then compatibility
and security review and hosted checks. Earlier assignments remain historical.

The isolated candidate uses Serde for the manifest, exclusively creates run
directories, checks manifest names and totals, and propagates Unix directory
sync failures. A baseline run of the focused test file at `534c9bd` had four
passes and three new failures; after the changes, the publication, Track 94
atomic-write and Track 70 durability test files passed 27 tests with Rust
1.99.1 and `--locked --features polars-native`. Focused Clippy and rustfmt
also passed. Build artifacts are in
`/Volumes/PortableSSD/codex-builds/kairos-arrow-publication-20261029`.
The portable follow-up candidate uses capability-based directory-relative,
no-follow opens via cap-primitives and cap-fs-ext 4.0.3, replacing the initial
Unix-only rustix approach. The deterministic directory-replacement and symlink
test passes on macOS; Windows execution is unverified. The new dependency graph
adds 22 locked packages, including target-specific packages, and its licence
and source checks pass after fetching all targets. Directory fsync is Unix-only,
and an error after rename may leave an already-published file visible. Writer
path handling under a hostile concurrent filesystem actor remains unqualified.
Independent security and compatibility review, cross-platform execution,
security audit, `just ci`, hosted checks and consumer acceptance remain open.

An independent synthetic readback of the separate zstd 0.14.3 workspace
qualification candidate found an additional contract gap. On 29 October 2026,
PyArrow 26.0.0 read its Parquet output with ZSTD columns and its Arrow IPC file
output, with equal logical rows and full-width tick limbs. PyArrow's IPC stream
reader rejected the output: `write_ipc_stream` currently uses Polars `IpcWriter`
and emits the IPC file format. Parquet strings and IPC strings also had
different physical Arrow types. Inputs, outputs, Rust producer and lock are in
`/Volumes/PortableSSD/codex-builds/kairos-zstd-independent-reader`; parquet
SHA-256 `17e474f2479b881860c2d1fe4c453dcac40b1ad080a08e0d86aff4b4fb5a64ad`,
IPC SHA-256 `76fd7129f6f68d7c4a57b2599fdc74e71dd9eadbb21cbff26803f29578a8d538`.
The apparent stream behavior and file/stream migration require contract and
consumer review; no independent stream interoperability is claimed.

An IPC repair candidate enables Polars `ipc_streaming` and changes the two
stream-named APIs to use the stream reader/writer. Previously produced IPC
files remain unchanged and can be opened explicitly with `read_ipc_file`.
This is a behavior change for callers that used the old stream-named APIs for
files; consumer migration and compatibility review are required before merge.
At base `8e385c6` with the IPC candidate, the seven focused Rust native-table
tests pass locally, including the stream marker and old-file reader. On macOS
ARM64 with repo-local Python 3.15.1 and PyArrow 26.0.0, an independently built
synthetic producer's output was read with `pyarrow.ipc.open_stream`; its one
logical row matched Parquet, including tick limbs 1:1. The stream SHA-256 is
`d0b3c5add601febbd4fc85ac7ff37b646ec5c7ba601fa96011d8f782e46d1297` and the
producer and output are retained in
`/Volumes/PortableSSD/codex-builds/kairos-ipc-independent-reader`. This is one
synthetic case, not cross-platform, hosted or downstream acceptance.

## zstd upstream and package readback — 29 October 2026

The workspace-only qualification candidate is retained in
`/Volumes/PortableSSD/codex-worktrees/kairos-zstd-compat-qualification-20261029`.
Its locally patched Arrow and Parquet crates resolve `zstd-safe 8.2.0` and
passed the recorded focused tests and cached advisory, source and licence
checks. It is not a distributable remediation: packaging `kairo-ecs-types`
and `kairo-ecs-arrow` together with `--offline --no-verify` succeeded, but the
actual Arrow archive's lockfile resolves registry `zstd 0.13.3` and
`zstd-safe 7.3.0`. The archive and exact receipt are retained at
`/Volumes/PortableSSD/codex-builds/kairos-zstd-package-qualification-20261029`.

Upstream Polars [commit e81e8cd](https://github.com/pola-rs/polars/commit/e81e8cd9068e0b8eae4c2b50f99058df33407dcc)
updated its development tree to the zstd 0.14 line. At readback, crates.io's
latest `polars-parquet 0.55.2` still declares zstd 0.13. The next supported
published version needs actual Arrow and Parquet dependency-tree, package,
independent reader, security, platform and consumer qualification; the
upstream commit or local workspace patch alone does not satisfy those gates.

## Local workspace validation — 29 October 2026

At `262e24300a5a5efabf52417781f3c5be37ace9eb` in the Arrow repair worktree,
`CARGO_TARGET_DIR=/Volumes/PortableSSD/codex-builds/kairos-arrow-publication-20261029 just ci`
passed formatting and workspace Clippy and ran 901 instrumented tests with
901 passed and none skipped. It exited 1 when its Homebrew Rust 1.99.1
(LLVM 23.1.3) toolchain could not locate `llvm-profdata`. Without rerunning
tests, `cargo llvm-cov report --lcov` successfully merged the retained profiles
with explicit Homebrew LLVM 23.1.4 tools, and the core coverage checker reported
1003/1074 lines (93.39%, floor 90%) in
`/Volumes/PortableSSD/codex-builds/kairos-arrow-publication-20261029/coverage.lcov`.
The separately run warning-denying workspace rustdoc and cached cargo-deny
advisory/source checks passed. `cargo audit --no-fetch --no-yanked` exited 1
with three zstd/zstd-safe vulnerabilities; yanked packages were not checked.
The original `just ci` did not pass, and these results do not establish Windows,
hosted, independent review or downstream package acceptance.

## Summary
All phases (Phase 0 through Phase 4) of Track 70 are complete and verified across all criteria:
- **Phase 0 (Baseline & qualification)**: Data path inventory, failing baseline characterizations, Polars dependency pin ADR, and Schema v2 conformance fixtures.
- **Phase 1 (Native tables & real formats)**: Strict Schema v2 Polars `DataFrame` creation (`records_to_dataframe`) and reconstruction (`dataframe_to_records`), full-range 128-bit tick preservation across high/low unsigned 64-bit limbs and borrow-preserving arithmetic (`subtract_tick_limbs`), real Zstandard Parquet and Arrow IPC stream writers/readers, and lazy Polars transform expressions (`filter_by_event_kind`, `filter_by_entity`, `aggregate_counts_by_status`).
- **Phase 2 (FFI & polyglot consumers)**: Populated 14-column Arrow C Data Interface schema and array exports (`kairo_ecs_export_event_log_arrow_schema`, `kairo_ecs_export_event_log_arrow_array`) with verified single-release lifecycles where exported buffers safely outlive engine mutation/destruction; native Python `EventLogBatch.to_polars()` handoff via PyArrow zero-copy interchange eliminating row-dict reconstruction; full backward/forward compatibility across Schema v1 and v2.
- **Phase 3 (Durable output & execution budgets)**: Bounded native output batches, manifest-last publication (`DurableRunPublicationWriter`), resume watermark sequence filtering without data loss or duplicate records (`DurableRunPublicationReader`), FNV-1a checksum validation rejecting tampered part files, and worker thread budget configuration (`get_or_set_execution_thread_budget`).
- **Phase 4 (Audit & CareOps handoff)**: Full test execution across Rust (`kairo-ecs-arrow`, `kairo-ecs-ffi`) and Python (`bindings/python/`), clean tree, documentation and changelog updates, and receipt manifest ready for CareOps adoption.

Track 70 is complete `[x]`.

## Follow-up issues
CareOps downstream integration: CareOps adopts the accepted Kairos Track 70 pin and consumes native Polars tables and Arrow C Data interface.

## Integration notes
Kairos delivery precedes parent CareOps pin and processing migration. Both planning and runtime contracts are integrated into the repository `main` branch. No release/push authority inferred. Minimal-core and Wasm feature isolation must continue to be verified on every phase change.

## Phase closeout evidence

- `$conductor-review`: Phase 4 review completed; native tables, extreme tick limb borrow arithmetic, real Parquet Zstd and Arrow IPC writers/readers, lazy transforms, populated 14-column Arrow C Data Interface Schema v2 exports, Python zero-copy handoff, and manifest-last durable publication verified.
- accepted fixes: Incorporated all schema validation, borrow limb subtraction, and single-release lifecycle checks across crates and bindings.
- commit SHA: `dc35ce9b8e8f80459c55b6a711f62cba394c2e6f` (Track 70 Phases 2-4 implementation); merged Phase 0 closeout PR #288 on `origin/main`; Phase 1 PR #289.
- pushed ref: `origin/main`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: verified clean tree state across phase closeout.
- next-phase decision: Track 70 is closed and approved for CareOps adoption.

## Sequencing revision — 23 October 2026

User-authorized planning change: supporting dependencies gate only demonstrated affected tasks. The existing implementation history and phase decisions above are preserved. No runtime paths, FMI work, pins, acceptance statuses or phase closeouts changed. See the Conductor dependency-sequencing.md decision contract.

## Current handoff structure — 29 October 2026

## Files changed

Arrow manifest/source, native filesystem guard, dependency locks and evidence.

## Contracts consumed

Arrow schema, deterministic full-width identity/ticks and protected publication contract.

## Contracts changed

Immutable Polars development source and hashbrown closure; source integration and crates.io gates are separate.

## Tests added

Native table/IPC, durability, protected publication, source/Git consumers and platform checks.

## Known risks

Crates.io publication stays disabled; final CI and package qualification remain required.

# Track 48 Handoff

Last updated: 2026-10-03

## Current local implementation and delivery boundary

Track 48 remains **In Progress**. The accepted local implementation adds the
`time-warp`-gated `OptimisticRuntime<P>` and PDES-owned `GenerationBitset` beside
the compatible scaffold helper. Complete model snapshots, structural full-key
ordering, surviving-input replay, recorded downstream-output anti-messages,
exact source-scoped incarnation cancellation, checked token authority, bounded
poison/failure handling and strictly-before-GVT fossil collection are implemented.
No core, ECS, debug, MPI or gRPC crate was changed by this slice.

The [API review](../../../docs/design/track48-api-review.md),
[ADR](adr-production-runtime.md) and
[contract](../../../docs/pdes/optimistic-runtime-contract.md) record preview
compatibility, model ownership, rejected designs and local review limits.
Fifteen held-out model/protocol tests and the combined 94-test crate lane passed
at `3cd6d56`. A subsequent compiler audit found Homebrew Rust 1.99.0 in both
independent Cargo target caches despite command wrappers labelled 1.98.1 and
1.76.0. Those results establish behavioral passes under the actual compiler;
they do **not** establish either claimed pinned toolchain or MSRV. Explicit
compiler-path reruns below now pass; the original receipts remain historical.

The source-bound sparse/dense benchmark at `ec9828e` has actual parity and five
raw alternating repeats in
`benches/pdes/evidence/track48-ec9828e/`. Its recorded wrapper toolchain metadata
is superseded for pinned compiler provenance by the accepted68b8d7a rerun below.
The benchmark isolates runtime run calls and excludes setup, state/report
extraction, validation and fossil collection. It is a small single-host fixture;
no general speedup, simultaneous CPU execution or distributed proof is claimed.

At delivery-review source `7a432ab`, the coordinator reports passing collector
nine-case validation, local manifest, phase, DAG and strict-clean gates. Root
`just ci` now passes with explicit Rust 1.98.1 PATH/RUSTC, matching LLVM tools
and a fresh target: 458 tests, zero skipped, core coverage 512/553 (92.59%)
against a 90% floor, formatting, all-target/all-feature Clippy and rustdoc with
warnings denied, cargo-deny advisory/source checks and cargo-audit all pass.
The coordinator's source-bound receipt/log are
`artifacts/track48-final-validation/receipt-resolved-pinned.json` and
`just-ci-resolved-pinned.log`. Earlier LLVM discovery/format failures remain
failed attempts, not pinned workspace evidence. The actual Rust 1.76.0 crate
rerun passed94 tests with matching Cargo/rustc1.76; canonical proof is preserved. No hosted Actions pass, push or merge is inferred.
A coordinator live-quality readback also reports a main-branch CodeQL/Scorecard
failure for alert482, including GitHub-reviewed braces advisory
`GHSA-vfj7-8cjw-p6xm` (affected<=3.0.3, no patched version in that readback),
missing Codecov project status despite upload success, and Renovate refresh
pending afterPR197. These are open cross-track/security evidence blockers;
no alert dismissal, bypass or extension of PR193-only EXC-193 is authorized.
The coordinator reconciled global narratives and release notes in d62891e/eb02a14.

Live distributed rollback artifacts, cross-participant GVT and the Track 49
integration gate remain required before Done. Track 49 still depends on
Tracks 35, 47 and 48. The [distributed handoff](distributed-interface-handoff.md)
is a proposal, with no production dispatch, dependency waiver or status advance.

## Historical scaffold record — June 2026

The following scaffold description and original commands record the earlier
helper slice. They are not the current event-owned runtime capability statement
or evidence of the resumed slice's compiler/hosted acceptance.

## Summary

Track 48 owns optimistic PDES and Time Warp rollback. The first implementation
slice adds a `time-warp` feature-gated local runtime primitive for straggler
rollback, anti-message cancellation, invalidated-send anti-message emission,
generation-checked component access, monotonic-GVT fossil collection, pre-GVT
straggler rejection, duplicate-positive rejection, and local overhead/rollback
pressure counters. The 2026-06-24 evidence-worker pass adds a local scaffold
manifest for that proof boundary without expanding the runtime claim to
distributed optimistic rollback.

## Files changed

- `crates/kairo-ecs-pdes/Cargo.toml`
- `crates/kairo-ecs-pdes/src/lib.rs`
- `conductor/tracks/48-time-warp-optimistic-rollback-runtime/*`
- `conductor/hpc-evidence/manifests/track48-local-time-warp-rollback-scaffold.json`

## Contracts consumed

- Track 47 production LP contract.
- Track 40 trace/replay semantics.
- Track 46 evidence manifest.
- Track 49 distributed transport boundary, consumed only as a blocked handoff
  line for future distributed proof.

## Contracts changed

The local `TimeWarpRuntime` now defines:

- `TimeWarpEventId` as the stable positive/anti-message match key.
- `TimeWarpEvent` with `Positive` and `Anti` message kinds.
- `TimeWarpStepReport` with rollback, canceled-event, and anti-message output.
- `TimeWarpComponentToken` generation checks for stale component access.
- `TimeWarpRuntime::fossil_collect(gvt)` with strict monotonic GVT advancement
  and pruning only for executed positive history older than GVT.
- `TimeWarpRuntime::overhead_metrics()` for local log, component generation,
  rollback, anti-message, duplicate-positive, and fossil-collection counters.
- `TimeWarpError` variants for stale generations, duplicate positives, GVT
  regression, and positive/anti-message arrivals older than GVT.

## Tests added

- `time_warp_straggler_rolls_back_to_prior_checkpoint`
- `time_warp_antimessage_cancels_matching_positive_event`
- `time_warp_generation_token_rejects_stale_component_access`
- `time_warp_fossil_collects_only_history_before_gvt`
- `time_warp_fossil_collection_preserves_rollback_at_or_after_gvt`
- `time_warp_rejects_pre_gvt_stragglers_and_gvt_regression`
- `time_warp_rejects_duplicate_positive_events_without_double_apply`
- `time_warp_overhead_metrics_track_local_rollback_pressure`

## Known risks

This is a local, dependency-free runtime primitive, not a full distributed Time
Warp scheduler. The current rollback model undoes future positives and emits
anti-messages for invalidated local inputs; it does not yet preserve a replay
queue for rolled-back future work or model downstream output anti-messages.
Optimistic execution beyond conservative safe time, benchmark evidence, live
HPC evidence, and distributed anti-message transport remain unimplemented. The
local evidence manifest
`conductor/hpc-evidence/manifests/track48-local-time-warp-rollback-scaffold.json`
is scaffold evidence only: it records local rollback/GVT/anti-message contract
coverage and an explicit `not-live` waiver. It does not prove distributed
optimistic rollback, cross-rank causality repair, replay redelivery,
downstream-output anti-message propagation, or live HPC execution.

## Follow-up issues

- Add replay-queue semantics for rolled-back future positives, or document the
  required Track 49 redelivery contract before any production Time Warp claim.
- Model downstream output anti-messages separately from canceled input events.
- Add rollback overhead benchmarks.
- Replace the local scaffold manifest with live distributed rollback evidence
  that records transport implementation, MPI/gRPC rank topology, raw artifacts,
  checksums, and reviewer acceptance before any `evidence-backed` Time Warp
  release claim.

## Integration notes

Any ECS storage changes require ecs-agent handoff before implementation. Track
49 can draft transport tests around `TimeWarpEventId` and `TimeWarpMessageKind`,
but distributed delivery remains out of scope for this slice. A valid
distributed optimistic rollback proof must come from Track 48 plus Track 49
integration and include at least: straggler delivery across process or rank
boundaries, deterministic anti-message routing for invalidated downstream
outputs, replay/redelivery of rolled-back future positives, GVT advancement
across participants, raw run artifacts, and checksums. The local manifest gate
validates evidence shape and claim boundaries only.

## Phase closeout evidence

Red step captured with
`rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features time-warp`;
the first run failed because `TimeWarpRuntime`, event, anti-message, and
generation token types did not exist.

Passing implementation gates:

- `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features time-warp time_warp`
- `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features time-warp`
- `rustup run stable-x86_64-pc-windows-gnu cargo test -p kairo-ecs-pdes --features pdes`
- `rustup run stable-x86_64-pc-windows-gnu cargo check --benches -p kairo-ecs-pdes --features time-warp`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo clippy -p kairo-ecs-pdes --all-targets --all-features -- -D warnings`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo test --workspace --all-features --jobs 1`
- `CARGO_INCREMENTAL=0 rustup run stable-x86_64-pc-windows-gnu cargo clippy --workspace --all-targets --all-features --jobs 1 -- -D warnings`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_phase_gates.ps1`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_dag.ps1`
- `node scripts/validation/validate-hpc-parity-evidence.mjs`

Initial full-workspace attempts without `--jobs 1` failed with OS error 112,
"There is not enough space on the disk." Removing generated build output and
rerunning with `CARGO_INCREMENTAL=0` and serialized jobs completed cleanly.

Implementation commit SHA: `35f93a4344615e2f8a4e5ca8a61ad7a483e87106`
pushed ref: `origin/codex/kairos-hpc-parity-wave`

2026-06-22 `$conductor-review` implementation pass for the fossil/GVT slice
recorded accepted fixes before closeout: explicit pre-GVT positive and
anti-message rejection, monotonic GVT regression errors, duplicate-positive
rejection, and bounded metrics for local rollback pressure. The review also
records replay-queue and downstream-output anti-message semantics as follow-up
risks rather than completed production Time Warp behavior. Strict closeout will
run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit
and push. The next-phase decision is to keep Track 48 In Progress until
optimistic safe-time execution, replay/redelivery semantics, benchmark evidence,
distributed transport, and live HPC evidence are complete.


## Resumption history — after Track 47 merge

PR #193 merged as fc2f7b7f8e24faef8d02e29aaaaaf64852945cf0 after all 19 exact-head workflow runs passed. Active branch is codex/kairos-track48-optimistic-runtime; parent checkout/pin remain outside this task. Architecture and ECS/trace reviews settled the bound contract in docs/pdes/optimistic-runtime-contract.md. No distributed/Done claim is made.

Actual RED baseline at source fc2f7b7: Rust 1.98.1 compiled three regression tests and exited 101. Initial seed restoration, stale-token recreation and anti rebuild each fail their independent oracle. Test source is integrated from d1e6472 as 1b3c0d8. Worker logs/receipt remain in /private/tmp/kairos-track48-regressions/artifacts/track48-red-baseline. One advisory lease token was mistakenly printed during a check; it was released immediately after commit and root verified no such lease remains active. No token is committed.

GenerationBitset source eb8a15153ac95666a3962e895ea6bfbbeb738191fc4b7a83c763ed00ae725a90 integrated at b4be30d. Worker standalone 11 tests pass on Rust 1.98.1; independent reviewer also ran all 11 on Rust 1.76.0 and 1.98.1 with exit 0. Module exports/full crate integration remain pending. Construction/restore allocation paths are source-reviewed; allocation failure was not fault-injected. Snapshot cloning uses ordinary Vec clone; no blanket allocation-safe claim is made.

The original Track 48/49 distributed acceptance cycle remains explicit. No Track 49 production dispatch or dependency change is authorized by this local packet. Current EXC-193 is PR #193-only; any future exception needs its own governed approval.

## Independent legacy repair acceptance

Legacy repair integrated from a71b654 as fd29a8382992356dff481b9647ab3c2f8b205a1c. Five fixed-fixture regressions failed before the repair and pass afterward. Independent reviewer reran all five on Rust 1.98.1 and 1.76.0, both exit 0; exact command/source/toolchain/log hashes are in local artifacts/track48-legacy-review/receipt.json. lib.rs SHA-256 d6a152d413e81fe90f83bc58fa1f6601c35d7890e6ef1438fa0a396245ab5589; regression source a841c06bf8c5ac0dd8834de8639000c126c9af212a04bc9206a10cff41472788. Independent source review found no blocker in the focused repair.

Retained limitations: post-first-event component writes are not transactional initialization; checked stamp exhaustion panics under the legacy nonfallible API; diagnostic generations remain saturating; seed baselines/canceled IDs remain retained. The helper still uses tick-only ordering and input anti-history, without replay queues or downstream-send ownership. New production guarantees must come from the separately reviewed optimistic driver. A second worker check briefly surfaced eight token characters; its complete token was not saved and the lease is now released.

The proposed distributed-interface-handoff.md is a draft only. No scheduling dependency, Track49 production authority, phase status, or live evidence gate has changed.

## Integration history — verification was pending at 3cd6d56

Integrated worker0709f2f and independent test drafts c5ca802/d48ce2d/79d0cfe into 3cd6d56ed98d39d311fe19f4d88857f43bfe6da6. The driver owns replay queues, complete process snapshots, actual downstream send logs, exact source-scoped incarnation cancellation, structural ordering, checked validity epochs and bounded GVT/failure transitions. The bitset module is now exported under time-warp. This is local implementation, not distributed proof or a Done claim.

Worker exact-source native time-warp tests, Clippy and formatting pass on Rust1.98.1; native full time-warp tests also pass on Rust1.76.0. Fifteen reviewer-owned oracles passed in a temporary worker debug run, without source edits; private absolute includes were removed before commit. Independent post-integration rerun, benchmark, combined-feature/workspace and hosted gates remain pending. Imported log/receipt/source hashes are recorded locally in artifacts/track48-integration/worker-proof-hashes.json; original logs/receipt remain under /private/tmp/kairos-track48-optimistic/artifacts/track48-optimistic/.

Review fixes include global exact delivery metadata, legitimate changed replay metadata, preflighted batch epochs/capacities, canceled replay marker cleanup, and maximum-frontier GVT lag. Five private exhaustion/batch-transition tests supplement eight worker integration tests and fifteen independent state/RNG/bitset/replay/failure oracles. Recovery preserved the interrupted source in stash bbbf38f96f55a6c1dcf57af91f10ad0956eb5358; the restarted lease and final writer lease are released.

## Local acceptance record and compiler-audit correction

At `3cd6d56`, all 15 reviewer-owned held-outs passed unchanged under two nominal toolchain wrappers; the combined pdes,time-warp lane passed 94 tests. The later Cargo-cache audit identifies actual Rust 1.99.0 for both targets, withdrawing the pinned 1.98.1/MSRV 1.76.0 labels pending explicit compiler-path reruns. Independent source review accepts bounded local semantics with strict-future outputs, ancestry<=128, complete owned model snapshots, no external handler side effects and caller-proven GVT. Receipt/hashes: /private/tmp/kairos-track48-independent-runtime/artifacts/track48-independent-runtime/receipt.json.

Benchmark integration ec9828e folds d1d1467/39a8e1. Review fixes isolate runtime run-call timing, define first-attempt/extra/replay counts, namespace committed IDs by actual source, validate fixed workloads and reject dirty/same-HEAD source edits. Nine collector negatives/tests pass; Rust1.98.1 and1.76.0 bench checks plus pinned Clippy pass. Root strict collector exit0 at clean ec9828e, with identical before/after source hashes; independent reviewer verifies artifact7c4b1e7a5de973fbbd7e7b5c1a6509d256785c7e3dc087008151b19321fe23e5. Canonical raw files retained in benches/pdes/evidence/track48-ec9828e/. One warmup and five alternating repeats per four cases; limits and excluded costs are explicit. These small single-host measurements do not prove general scaling, actual simultaneous CPU execution or distributed rollback.

Merged accepted upstreammain PR197 as0ab0c56; its two documentation workflow mitigations and changelog entry do not overlap runtime source. Final workspace/Conductor/hosted gates remain pending. No phase/track Done status is advanced.

## Explicit MSRV evidence accepted — 2026-10-03

At source7a432ab, matching Cargo/rustc/rustdoc1.76.0 with explicit PATH/RUSTC/RUSTDOC and fresh target passed94 local pdes,time-warp tests plus doc tests. Independent review verified all bound source hashes, actual Cargo compiler cache and successful log SHA c9bca00d9a453414809181c1950294a4189169afd601ce77d77350dfd3078b01. Canonical unchanged copies are in benches/pdes/evidence/track48-msrv-7a432ab/. This resolves the local MSRV proof gap; historical compiler labels remain withdrawn. Benchmark compiler correction, exact-head hosted/security and live distributed gates remain pending.

## Final compiler-bound local delivery — 2026-10-03

Reviewed collector504aa4a integrated as68b8d7a. Absolute Cargo/rustc1.98.1, binary/configuration hashes, wrapper/selector refusal and fixed baseline flags resolve compiler drift. At clean68b8d7a,12 collector tests and strict fresh benchmark passed; independent review verified actual Cargo cache1.98.1/LLVM22.1.8,16 unchanged source hashes, raw logs and four parity cases with five alternating samples. Canonical evidence: benches/pdes/evidence/track48-68b8d7a/, SHAe7d08f74b0c448cac281cd95199e24cf9d34218a303a80ad0a66f8e1323cbc8c. Scope is tiny single-host lightweight-handler run-call smoke, with substantial timing variation and excluded setup/extraction/validation/fossil costs. No general speedup, CPU-concurrency or distributed claim.

Explicit1.98 workspace CI458/coverage92.59% and matching1.76 local PDES94 tests are accepted. The workspace doc-test command at7c35f2a exited0 across25 targets with zero runnable examples, separately preserved in benches/pdes/evidence/track48-doctests-7c35f2a/. CHANGELOG d62891e and global evidence synchronization eb02a14 are committed; previous pending statements describe historical review snapshots. Exact-head push/hosted security/package gates and live distributed acceptance remain pending. Track48 stays In Progress; Track49 dependency/production authority unchanged.

During compiler-fix coordination, an expired worker lease was explicitly recovered only after interruption; exact edits were preserved in stashes. A displayed cooperative lease token was revoked and rotated before continuation. No displayed token is committed or remains active. Original failed attempts and superseded raw evidence remain preserved.

## Final local gates and forward workspace MSRV

At c87df47, pinned1.98 benchmark compilation,12 collector tests, local manifest, phase/DAG and strict clean Git checks pass; immutable copies are in benches/pdes/evidence/track48-local-gates-c87df47/. Matching absolute Cargo/rustc/rustdoc1.76 also passes locked all-feature workspace lib/bin check excluding Wasm; actual compiler cache and log verified in benches/pdes/evidence/track48-workspace-msrv-c87df47/. This forward evidence resolves current compiler compatibility without retrospectively asserting compiler identity for old wrapper-labelled Track47 MSRV receipts. No Windows, foreign-package or distributed proof is claimed.

## Draft PR199 hosted readback

Branch pushed and PR199 opened at0a3b86aa33d1f158eaca2855e0e503cdac6e08ef: https://github.com/edithatogo/kairos/pull/199. Native stable/MSRV, bindings, benchmark, docs and CodeQL lanes passed at that exact head. Conductor validation failed on missing historical Git objects and the worker optimistic test's absent time-warp feature guard. npm-package correctly rejected reuse of EXC193 on PR199 before executing raw audit; retained hosted receipt is in artifacts/track48-pr199-blockers/hosted-audit/. These are failed gates, not acceptance.

Fresh local bootstrap raw audit exit1 retains19 high findings, empty stderr and exact previously reviewed graph SHA0b3e5f1d5f65b48f1a20618ba352e6f02529a134f62e0230126ac68c73b5fec8. Current installer/resolution and both60-case behavior checks pass;194 package signatures verify. Raw commands/logs are retained under artifacts/track48-pr199-blockers/ and artifacts/track48-pr199-controls/. EXC193 is not extended. A separate199-only human decision remains required if an operational exception is proposed.

Accepted website PR198 merged as upstreama481cb2 and was integrated as e3306f4; no native runtime source changed. Track48 remains In Progress with distributed acceptance and Track49 scheduling authority unchanged.

## PR199 reviewed Conductor repair and pending exception

Worker e008d78 integrated as b6b4943 adds the missing time-warp test guard, full-history checkout and Track13 handoff. Explicit1.98.1 default workspace305 tests and time-warp79 pass after reproduced default-feature E0432; immutable local captures are in benches/pdes/evidence/track48-conductor-repair-e008d78/. Exact-head hosted rerun remains required.

Pending EXC199 packet integrated as ece2306 includes fresh audit and compensating-control evidence; approval fields remain unset. Its decision record is conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions/EXC-199-http-cache-decision.md. No classifier/runner/workflow activation or change to EXC193 scope was made. This is a concrete proposal for human Security/Release owner review, not an approved exception.

## Independent wire preparation — 3 October 2026

Reviewed draft requirements and deterministic reference fixtures are in [wire-draft/contract.md](wire-draft/contract.md), with source-bound packet and executed receipts alongside. Seven local reference tests and phase gates passed; these test delivery membership, structural identity, bounds and necessary GVT/fossil rules only. Actual codecs, native rollback/RNG parity, durable authority epochs, migration, receiver acknowledgements and live MPI/gRPC remain unimplemented/unverified. Existing protobuf gaps are documented without modifying Track49 files. Track49 production scheduling, Track48 Done, EXC199 approval, shared CI/manifests and the parent pin remain unchanged.

Wire draft review fixes accept native incarnation zero, validate every ancestor tick and preflight bounded structural types before serialization. Ten local reference tests pass after those corrections. Deep/cyclic and invalid scalar cases fail before membership mutation; no decoder or allocation safety certification is implied.

Independent read-only review accepted draft source3008b288 on 3 October2026 after verifying the ten-check/phase receipt, committed input/output hashes and closure of all three findings. Final review is recorded in wire-draft/receipt.json. The isolated codex/kairos-track48-wire-drafts branch remains a prepared satellite; it is not integrated into PR199 and does not disturb its hosted checks or parallel work.

## Approved conditional Track49 entry — 4 October2026

The human sole maintainer explicitly approved Track49 production scheduling after scope clarification. See conductor/tracks/29-wave-manager-execution-gatekeeper/adr-track49-phase-entry-20261004.md. Dispatch remains conditional on normal PR199 merge, exact-head gates/security evidence, accepted local/interface proof and a separately reviewed reserved transport packet. Track48 remains In Progress; dependencies, full distributed acceptance and release gates are unchanged. The raw Track49 gate result is preserved, not rewritten as a pass.


## Codec bridge prerequisite — 4 October 2026

PR199 merged normally as786f50b after exact reviewed-head52d8 checks; the merged tree equals that head. Current [entry readback](codec-bridge-entry/receipt.json) retains live GitHub merge/check evidence. No protection bypass or whole-track completion occurred.

Architecture and independent hostile-input roles accepted the [codec ADR](adr-codec-bridge.md). Luna's scoped worker2dbdd1b adds the three public bridge methods and four integration fixtures; root c19fbf4 integrates the exact source/test blobs. Independent architecture source/receipt review accepts this bounded local prerequisite. Explicit1.98.1 worker98 tests, Clippy/format and matching1.76.0 root98 tests pass; canonical [provenance](codec-bridge-evidence/README.md) retains actual source hashes, compiler caches, logs and earlier failed fixture/format attempts. Independent held-out fixture acceptance, final combined/full checks, PR creation and exact-head hosted acceptance remain pending.

The public runtime contract and API compatibility assessment now document the additive alpha bridge and native u128 preservation. Preparing the next real distributed steps revealed two additional native prerequisites: [source authority identity](authority-identity-design-candidates.md) and [owned-process/accounted outbox routing](owned-outbox-design-prerequisite.md). Epoch must reach native cancellation/queue identity, and remote positives/antis must remain accounted until durable admission. Observer copies and local all-LP execution cannot substitute for real transport. These are proposal records, not new native interfaces or dispatch authority. Track48 remains In Progress; Track29 conjunctive conditional entry and other dependencies remain unchanged. EXC199 is not approval for a new PR's audit gate.


Codec bridge combined local acceptance: integrated b8ff195 passes all five independent held-outs on explicitly bound Rust1.98.1 and1.76.0; the independent reviewer verifies source, raw log and actual compiler-cache hashes. Matching1.98.1/LLVM22.1.8 `just ci` passes467 tests, zero skipped, core coverage512/553 (92.59%) and fmt/Clippy/rustdoc/deny/audit. Architecture role independently accepts source/code/docs provenance. Canonical codec-bridge-evidence records preserve these actual snapshots. Hosted/new-PR, authority/fencing, owned/outbox, durable admission and actual distributed gates remain pending; Track48 stays In Progress.

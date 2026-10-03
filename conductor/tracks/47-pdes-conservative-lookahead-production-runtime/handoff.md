# Track 47 Handoff

Last updated: 2026-10-03

## Summary

Track 47 adds an event-owned conservative single-host Rust runtime behind
`pdes`. Real scoped OS workers execute independent LP inputs. Runtime queues,
directed topology, positive lookahead, exclusive safe-time bounds, atomic
outbound validation and monotonic GVT are owned by the coordinator. Track 34
callback APIs remain compatible and explicitly labelled as scaffold surfaces.
The source has passed focused native correctness gates. Full Rust/Conductor
closeout, immutable pushed-source hardware evidence and hosted Actions remain
separate recorded gates below.

## Files changed

- `crates/kairo-ecs-pdes/src/conservative.rs` and feature-gated exports in `src/lib.rs`.
- `crates/kairo-ecs-pdes/tests/production_parity.rs` and `production_protocol.rs`.
- `crates/kairo-ecs-pdes/benches/production.rs` and crate dev/bench configuration.
- `Cargo.lock`: existing core/serde_json dev dependencies only, under root ownership handoff.
- `conformance/fixtures/pdes_conservative_parity_v1.json`: dedicated additive fixture, under root ownership handoff; root registers its manifest/runner metadata.
- `benches/pdes/`: raw repeated timing collector, evidence validation and profiles.
- `docs/pdes/`: runtime contract/migration, compatibility labels and benchmark evidence.
- Track 47 ADR, review, plan, spec, risks and test matrix.

## Contracts consumed

Consumes Track 34 shared event/partition types, Track 01 public Scheduler as the
actual sequential oracle, Track 46 hardware manifest and Track 31 measurement
boundaries.

## Contracts changed

New preview Rust API: `ConservativeProcess`, `ConservativeRuntime`,
`RuntimeError` and `RuntimeReport`. No ABI, language binding or Arrow schema
changes occur. The ADR records compatibility assessment and red-team responses.

Track 49 receives the LP, event and directed exclusive-channel-bound contract
in `docs/pdes/production-conservative-runtime.md`. Its distributed transports
must prove in-flight accounting, channel ordering and failure behavior.
Track 55 receives true runtime event/emission/null/round/worker/GVT metrics and
raw strong/weak host measurements; oversubscription is recorded. Core scheduler
internals need no handoff request or changes for this implementation.

## Tests added

The shared versioned fixture executes 18 deterministic DES/ABM/mixed
combinations (2/8 LPs, seeds 7/47/24301) through both the real core `Scheduler`
and production runtime. Fourteen production protocol tests include an actual
8-LP minimum-lookahead cycle with exactly 10,001 handled events over 10,000
simulated ticks, randomized positive delays, sparse future input, same-LP
reinsertion, exact-bound waiting, atomic rejection, panic/poison, event budget,
typed overflow and ownership/route preflight. Legacy tests remain distinct.

`review.md` records the substantive draft findings and accepted corrections.
No high-priority source finding remains from that review. No historical failing
TDD log or hosted pass is inferred from existing checked helper tasks.

## Executed local evidence

Working directory for all commands:
`/Users/doughnut/Documents/careops-sim/.worktrees/kairos-implementation-programme`.
Toolchain: native `rustup run 1.98.1` (macOS); source initially uncommitted on the
programme branch and then captured by the final scoped source commit.

- `cargo test -p kairo-ecs-pdes --features pdes --test production_parity --test production_protocol` via Rust 1.98.1: exit 0, 18 model combinations inside 1 parity test plus 11 protocol tests at that review checkpoint.
- `cargo test -p kairo-ecs-pdes --all-features` via Rust 1.98.1: exit 0, 31 unit + 1 parity + 13 protocol + 4 legacy integration tests, before the final added typed overflow case.
- `cargo test -p kairo-ecs-pdes --no-default-features` via Rust 1.98.1: exit 0, 20 unit + 4 legacy integration tests; production tests correctly gated out.
- `cargo test -p kairo-ecs-pdes --features pdes --test production_protocol emitting_event_lookahead_overflow_is_typed_and_poisoning` via Rust 1.98.1: exit 0, final added typed overflow/poison case.

The final full lane, collector/profile artifact paths, source SHA, pushed ref,
hosted checks and clean closeout are appended by the coordinator only after
execution. Initial draft benchmark import/collector syntax failures were
review findings, not accepted gate evidence.

Fixture input SHA-256:
`e63503111ba2aedeaf3b76a302cbd5c25d1964f8eac91734d4cbc84d351214dd`.

Root executed the final pinned `just ci` lane (toolchain PATH points to Rust
1.98.1 for cargo/rustc/clippy/rustfmt): exit 0, 414/414 workspace tests, core
coverage 92.59%, formatting, lint, docs and dependency policy checks passed.
Raw log: `artifacts/track47-root-ci.log`. Root also ran the Rust 1.76 workspace
MSRV check excluding the independently gated Wasm crate: passed. This is native
local evidence and does not imply hosted Actions acceptance.

## Remaining closeout gates

- Conductor/DAG/HPC/clean Git checks on final committed source; any subsequent source fix gets relevant repeated validation.
- Strong/weak 4/8/16/32-LP hardware capture with actual topology/cores/seeds,
  exact pushed commit/ref, immutable raw bytes and sha256 manifest.
- Hosted Actions on the exact reviewed PR head, then synchronized accepted
  closeout metadata and merge before Track 48 implementation.

## Known risks

This is single-host CPU/thread evidence. Track 49 distributed execution,
Track 55 comparative HPC certification and platform resource-failure fault
injection are not claimed by local gates. Per-round worker creation overhead
is measured. A poisoned runtime cannot recover partially mutated callback
state; budget exhaustion is explicitly resumable. Default event budget is one
million events per call and callers can supply another explicit bound.


## Follow-up issues

No in-scope runtime correctness issue remains from source review. The raw
hardware manifest and hosted closeout are open acceptance work, owned by the
root coordinator. Track 55 comparative certification and Track 49 distributed
transports retain their own gates.

## Integration notes

All source changes remain within Track 47 except the root-authorized additive
shared fixture and existing locked dev-dependency handoffs. No blocked core
scheduler or rollback internals changed. No later track executes concurrently.

## Phase closeout evidence

`$conductor-review` was applied to the actual source/fixture/benchmark diff;
`review.md` records accepted fixes and the root independent review.
Commit SHA: pending the scoped source commit; the coordinator appends the
executed value after commit. Pushed ref: pending root delivery.
`pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`
remains required after the final commit and push; no pass is asserted here.
Next-phase decision: keep Track 47 open until raw pushed-source hardware
capture, Conductor synchronization and exact-head hosted Actions pass; merge
its PR before Track 48 begins. The capitalized next-phase decision field and
recorded commit SHA are filled with executed evidence at final closeout.


Standalone phase-gate validation after the handoff repair:
`pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, exit 0,
0 errors and 0 warnings. Collector evidence-boundary self-checks:
`python3 -m unittest benches.pdes.test_collect_evidence -v`, exit 0, 7 tests
passed including first-row porcelain preservation, dirty non-owned source,
concurrent source drift, invalid input matrix, parity and counter drift.

Optimized benchmark smoke after Cargo's appended `--bench` harness flag was
accepted: `rustup run 1.98.1 cargo bench -p kairo-ecs-pdes --bench production
--features pdes -- --seed 472026 --repetitions 1`, exit 0, all eight strong/weak
4/8/16/32-LP rows returned parity true, exact event counts, GVT 1 and workers
matching LP count. Review-only raw output: `artifacts/track47-bench-smoke.json`
and `.stderr`; these are not the pushed-source Track 46 manifest. Targeted bench
formatting and clippy passed after that sole post-full-CI parser adjustment.


Source commit SHA: `71acd990692a2cadd4cfea9c1f7396994ea81109` was pushed by the
root coordinator. The first attempted hardware capture refused a dirty
collector before benchmarking and created no evidence bundle. A narrow
collector follow-up replaces BSD `stat %T` (which reports file kind) with actual
source-volume `diskutil` filesystem-type readback, removes local mount/device
paths from public metadata, requires an explicit nonblank live reviewer, and
fails metadata commands closed. Direct metadata check observed APFS; collector
py_compile, all 7 evidence-boundary self-checks and diff integrity passed.
The root coordinator retries capture only against the next clean pushed commit.


Acceptance-audit metadata follow-up: collector records repository-relative
`working_directory` (`.`), the completed child `benchmark_exit_status`, and
`input_scenario_sha256` over canonical sorted compact UTF-8 scenario JSON.
Its scope includes seed/profile/LP and repetition/warmup parameters, with the
payload generator defined by source commit; it is not an event-payload digest.
`worker_count` is the maximum spawned LP cohort in a round, not measured
simultaneous CPU execution. First capture
`benches/pdes/evidence/20261003T023344Z-77797709da-5f5d46e7` is preserved unchanged
while the root captures the final audited metadata against the next pushed
commit. Eight collector self-checks passed, including metadata JSON round-trip,
canonical key-order stability and changed-seed hash sensitivity. Pinned Rust
1.98.1 targeted crate formatting and all-target/all-feature clippy passed;
only a Rust documentation comment changed, with no runtime behavior change.

## Final source and hardware evidence - 2026-10-03

Source commit SHA: `45f01c49be1d15cefb64ad48e59ee0a7e4146b3e`; remote readback confirmed the same SHA before capture.

Executed from the repository root: `python3 benches/pdes/collect_evidence.py --commit-sha 45f01c49be1d15cefb64ad48e59ee0a7e4146b3e --pushed-ref refs/remotes/origin/codex/kairos-implementation-programme --evidence-class live-hpc --seed 472026 --repetitions 5 --reviewer "Codex Track 47 substantive review"`; exit status 0.

Registered evidence: `conductor/hpc-evidence/manifests/track47-conservative-production-live.json`. The raw artifact, actual compiler/host/core/topology, scenario hash, working directory and child exit status are recorded in that manifest. All eight profiles pass parity. The raw checksum is `sha256:69003d380f88dfe48249ca02d3beff8da18ae0f7c30f989976196a03c12e121c`. Initial capture is preserved but superseded for acceptance.

Track status is In Review. Hosted Actions and strict clean pushed git closeout remain pending; the next-phase decision is to wait for Track 47 merge before Track 48.

## Hosted acceptance blocker - 2026-10-03

PR: https://github.com/edithatogo/kairos/pull/193. At head
`2502a4cce8526af74efc99a5ffedb1a2b6156cfb`, 15 PR Actions runs passed and
one failed: Package Dry Runs run `37090747480`, npm-package job
`111110374306`. Its required moderate-level npm audit exited 1 with 19
high-severity dependency/metavulnerability nodes rooted in
`npm@12.1.0 -> make-fetch-happen@16.0.1 -> http-cache-semantics@4.2.0`.
The affected package is registry-resolved outside npm's repacked embedded bundle.

Reviewed advisory: https://github.com/advisories/GHSA-ch52-4w7c-c8xp.
It reports affected versions through 4.2.0 and no patched version. Independent
registry/source review on 2026-10-03 found npm 12.2.0 still includes the same
affected cache package. The upstream fix at
https://github.com/kornelski/http-cache-semantics/pull/58 remains open and unmerged
(head `14a8c2ad51740dc39bf3e8f1a11c845a5003f217`). No dependency upgrade,
advisory suppression, fabricated package version, or gate removal was applied.
A published fixed upstream candidate and successful existing audit are needed
before final acceptance; Track 48 must wait for Track 47 merge.

The two substantive PR review fixes are implemented: portable Linux kernel
core topology/model evidence, with unavailable topology preserved, and a
synchronized In Review spec. Root independently ran the 13-test collector suite
(exit 0) and verified the actual Mac hardware readback. The bounded Track 13 CI
handoff retains benchmark checks and adds those collector tests plus an actual
Linux runner hardware readback, with no new actions, dependencies or permissions.
Exact-head hosted verification for these follow-up changes is still pending.

## HTTP cache local source mitigation — 3 October 2026

User-authorized security follow-up owns the bootstrap-node-tools source patcher and validator, shell bootstrap invocation, and Track 13 package workflow wiring. This cross-track handoff does not change the Track 47 runtime or the published dependency identity.

The patcher accepts only released http-cache-semantics@4.2.0 index SHA-256 `01b7d66c854b2fe53ac05c98feb6e0d64722ab8898a778e2d2426a8b468d178f` and exact patched SHA-256 `fc7b3f0265b7a7d0fee83bafa47186a66495720d3179801c2be3083de6d0cf76`, from upstream commit `14a8c2ad51740dc39bf3e8f1a11c845a5003f217` ([PR 58](https://github.com/kornelski/http-cache-semantics/pull/58)). Package name/version, registry lock integrity and upstream BSD-2-Clause license remain unchanged. Replacements are atomic per file; errors fail closed before npm consumers run. This execution path is verified locally on macOS; Windows patcher behavior is unverified.

Seven installer tests and 60 security/compatibility cases pass locally on Node 26.10.0; the full behavioral suite fails on the released source as an independent negative control. npm and make-fetch-happen must resolve the exact patched bytes. Local command receipts are in artifacts/http-cache-security-fix/receipt.json. Hosted Node 22.22.2 results remain pending. The raw npm audit and signature commands are retained unchanged: the advisory still marks version 4.2.0 affected, so audit/merge acceptance remains blocked until an official fix or a separately authorized governed exception. No exception is introduced here.

Hosted mitigation readback: [Package Dry Runs 37094455626](https://github.com/edithatogo/kairos/actions/runs/37094455626), source `b68a5c780fe6dc58713fb3adaa6a85d5e6748a62`, Linux / Node 22.22.2: seven installer tests pass, both dependency copies have the pinned patched hash, both 60-case behavior suites pass, and consumer resolution passes. The npm job then fails the unchanged raw audit with no published fix. License attribution is corrected to BSD-2-Clause; the upstream copyright and terms are retained in the patcher. These are mitigation results, not an audit pass or merge acceptance.


## Approved EXC-193 activation preparation

The human sole maintainer approved EXC-193 in the current chat, acting in both security and release owner roles. This supersedes the earlier requirement to wait exclusively for a published fixed version. The narrowly bound exception permits PR #193 development integration and alpha/beta package dry runs only, until 2026-10-10T00:00:00+10:00. RC, 1.0 and publication remain excluded.

The runner verifies immutable proof hashes and both installed patched copies, executes mitigation checks, retains raw audit stdout/stderr/exit and applies the exact advisory graph classifier. Initial local execution at 750a8d2 returned approved_temporary_exception with raw audit exit 1; this is no claim of a clean audit. Eleven local filesystem/context negative controls passed. Independent integrated tests exposed a Python test descriptor binding error; correction and fresh final-head execution are pending. Tracked runner adversarial tests and hosted verification are also pending. No merge or completed hosted gate is claimed here.

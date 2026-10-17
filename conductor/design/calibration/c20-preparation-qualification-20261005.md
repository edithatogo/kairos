# C2.0 preparation qualification — 5 October 2026

Status: local definition/preparation review completed; hosted publication and
parent task closeout pending. Runtime acceptance remains open.

## Scope and source

Integrated source: `4f0ca5c7e3e267fd4a5889ba8a643f228abcd147`. Baseline is
`65858adfff7976f90802f9cda20aa34b6fd64d90`. Changes since baseline are frozen
contracts, an experimental API adoption decision, six fixture families and their
native runners. Production Rust, Cargo manifests and lockfiles are unchanged.
The companion [receipts](c20-preparation-receipts-20261005.json) bind all six
executed checks to that exact source, toolchain, commands, fixtures and log hashes.

## Definition review

Provider validation covers typed errors, canonical IDs, normative draws,
rejection sampling, lossless durations, owned identity and continuation.
Routes cover integer units, cumulative rounding, overflow, canonical geometry,
zero legs/cycles and full-sequence deterministic ties. Planned hooks cover
staged context/command atomicity, error precedence and existing mutable behavior.
Paired Flow tests require actual empirical Service draws, Macro/zero-Micro
outcomes, no transit and no resampling. Transit tests require owned controls,
stale start/arrival handling, repeated-control planner rejection, unchanged
paused progress, actual arrival claim and completion-record accounting.
Suspend/Restart tests require original sample/template accounting and unchanged
completed carrier identity/progress.

Independent reviews by d35_gate_review, d35_evidence_review and c20_route_red
were bounded and read-only when reviewing another writer's changes. Findings
were corrected: undefined helper/arity, consuming Bound ownership, noncontractual
observation equality, reversed urgency priority, missing Restart carrier and
actor-binding oracles, cleared completion timestamp, repeated Pause receipt,
postcommit runner base and filtered-test/summary mismatches. Final evidence review found missing hook stdout/stderr hashes; both actual output
files are now hashed in the companion record. No objection remains on the final
definitions or preemption runner. Reviews establish test intent;
missing APIs prevent typecheck and behavioral proof.

## Executed integrated checks

All six runners ran with `--expect red` from the Kairos checkout on the source
above, using canonical Rust 1.99.0. Each wrapper returned zero after native Cargo
returned 101 with only its declared missing-API diagnostic. Provider and paired
fixtures stop at a real absent production file. Routes identify absent spatial
exports; hook diagnostics identify absent planned hooks and typed controls.
The missing-file failures prove absence only, and cannot qualify future APIs.
Each implementation must run every required named test, with no ignored tests
or weakened oracle. Failed intermediate worker receipts remain preserved locally.

## Remaining acceptance and implementation gates

1. Publish/review the preparation branch and verify exact-head hosted checks;
   reconcile parent evidence/pin under its own claim before checking C2.0 complete.
2. Bind C2.2 duration/policy/seed packets to the reviewed contracts and actual
   production source. Implement real owned streams/provider/admission; preserve
   scheduler/RNG goldens and compatibility floors.
3. C2.3 implements production routes, transactional planned hooks, controls and
   transit execution; C2.1 joins runtime paired/lifecycle evidence, then C2.4 reviews.
4. Production exports/features require ADR-0007 compatibility/MSRV qualification.
   Default DES/ABM stay Rust1.76; optional calibration/Arrow stay Rust1.88.
5. Portable checkpoint rebinding, stable API release, ED MVP and v1 remain separate.
   Preserve parallel Track49 and C4 ownership. No renderer/private EHR scope added.

No C2 task checkbox, upstream completion record, release or runtime capability
is changed by this qualification record.

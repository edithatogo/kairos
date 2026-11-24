# Handoff

Updated 25 November 2026. Status **Done**.

## Qualification and claim validation — 25 November 2026

The shared register generates the claim table used by the documentation and
release review. The new run checks bind capability IDs, source and target to
typed receipts and recheck the retained Arrow bytes. Benchmarks, native
execution, empirical evidence and frontier results require their respective
evidence classes. Each source-specific receipt carries its original revisions
and limits.

Node 26.13.1 accepted all 27 current entries. Twenty conformance cases
passed in 1.41 seconds on Apple M1 Max. The published-command test exercises
generation, drift and recovery. The review of the scientific inputs and
native execution remains in the linked records.
[Generated table](../../capabilities.md) ·
[Native receipt](../110-scientific-method-completion/evidence/native-qualification-20261125.json) ·
[Guard tests](../../../tests/conformance/frontier-evidence-check.mjs)

## Current claims and reused evidence — 23 November 2026

Integration chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns the capability
register, its guard tests and the Python candidate qualification row in
the Kairos closeout worktree on `codex/kairos-capability-evidence-20261123`.

The register identifies the installed Rust SDK, C ABI v1, CPython 3.15.2
and CareOps contention cases by source and target. The two receipt files
in this track are the exact bytes from
[run 45051171243](https://github.com/edithatogo/kairos/actions/runs/45051171243).
The existing `packaging/scripts/verify_native_beta_evidence.py` verifier
matched the retained bundles and receipts to the selected candidate and
its two profile targets. Track 113 owns the maintainer's acceptance decision.

[Release Attestations run 45053462090](https://github.com/edithatogo/kairos/actions/runs/45053462090)
signed the SPDX 2.3 SBOM for the installed candidate
`e2875387ce45c34cd19716ea66f0102251a6aa6f`. The attestation covers six
named subjects and five distinct archive digests across the two tested
targets. The SPDX SHA-256 is
`34e2bdcc6240d81caff241bb3400387683c8e556b0bf68ea987153b039513ba7`.

Current claims use the merged [scientific evidence](../../scientific-completion.md#qualified-s05s10-scopes)
for Sobol and sensitivity intervals, sequential ABC-SMC, common random
numbers, competing risks, affine inference and the experimental native queue.
The installed CareOps and Innovate records cover the named cases and
artifact revisions. The [matched conservative-PDES ring](../113-beta-and-stable-release/evidence/paired-pdes-e287538.json)
keeps parity over 120 paired samples and shows lower median elapsed time
for serial in all eight cases. The broader Track 111 crossover remains open.

The existing guard suite builds its fixture from the selected claims and
their declared evidence. It still rejects missing native proof, synthetic
performance or native promotion, altered timings, mismatched parity, invalid
track links and reversed dependency direction. A dangling receipt reference
also rejects. Independent review checked the claims and native receipt
bytes, confirmed the guard oracles and added the exact signed SBOM run
and the narrower inference label.

## Summary

Keep public claims traceable to precise evidence and prevent placeholders, historical receipts or interface-shape tests from becoming false acceptance.

## Files changed

The capability inventory and rendered Markdown, the claim validator and its
conformance suite, and the Conductor plan, test matrix and phase record.

## Contracts consumed

AGENTS.md, existing core/FFI/conformance/versioning contracts, the approved [programme](../../frontier-programme-20261107.md), Tracks 12, 13, 14, 18, 21.

## Contracts changed

The programme establishes immediate Rust-only support; this track qualifies its own specific outcomes. No release/publication or downstream-pin acceptance follows from track registration.

## Tests added

The [test matrix](test-matrix.md) records drift handling, receipt tamper
controls and attempts to promote claims using unrelated or untyped evidence.

## Known risks

A schema validator cannot prove a reported experiment occurred. Require actual raw execution evidence and independent review; never turn metadata success into performance, clinical or release acceptance.

## Follow-up issues

Require new capability claims to carry their own reviewed receipts. Tracks
110, 113 and 115 retain method, release and research acceptance.

## Integration notes

Integration chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` owns the register, guard and documentation
reconciliation in `/Users/doughnut/.codex/worktrees/kairos-scientific-evidence-closeout/kairos` on `codex/scientific-evidence-closeout-20261125`. Hosted integration checks
bind the final tree to the reviewed guard. The earlier owner assignment
and installed receipt identities are retained above.

## Phase closeout evidence

Independent `$conductor-review` accepted Phases 0–2. The accepted fixes
bind the native-core-v1 profile, restrict the historical Track 49 record
to PDES and preserve the benchmark and frontier evidence requirements.
The guard commit SHA is `f2f5197c7a1eaa05bb0f627292a2afd84f8d5c58`. The pushed ref is `origin/main`.
Run `scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`
against the integrated tree. The next-phase decision closes this track.
Future evidence uses the same receipt and generated-table path.


# Handoff

Updated 23 November 2026. Both advertised targets passed the installed
candidate gate. Phase 0 remains open for final candidate review and
maintainer acceptance.

## Native core v1 candidate — 23 November 2026

`native-core-v1` selects main commit
`e2875387ce45c34cd19716ea66f0102251a6aa6f`. The Apache-2.0 Rust SDK, C ABI
v1 archive and installed CPython 3.15.2 wheel passed on Linux x86-64 GNU and
macOS ARM64. Both targets rejected missing selection, wrong architecture,
missing symbols and an incompatible ABI. Both executed the installed CareOps
contention case in the profiled environment.
[Hosted installation and exact archives](https://github.com/edithatogo/kairos/actions/runs/45051171243).

The shared SDK SHA-256 is
`0060c934d7b5d9eb187387175fc4604184093823aedbfbb43fa52936b1a82240`.

| Target | Hosted installed receipt SHA-256 |
| --- | --- |
| Linux x86-64 GNU | `bdf06ca4ece1be84f63cda9793b9a9be5634e9b1e3dcd8a84aef3a50bb7ced88` |
| macOS ARM64 | `3a871e601f5da415758ad3f47691853d5e39a67993295cd3a731256964bfd7b1` |

### Candidate compatibility, performance and provenance

- **Compatibility.** CareOps `9af99b5730d6aa5cef1c05bd48e90fd377b80cf2`
  admitted and reproduced its historical nominal run byte for byte using
  the downloaded SDK and a locked Rust 1.99.1 build. Test log SHA-256
  `b6b5b3ecc7fe6f89cfb4e12da87646bd2d025f0f068ae44acc4a0835dc9c31bc`.
  The downloaded SDK also passed the three unchanged alpha scheduler
  fixtures and the candidate RNG fixture. It reproduced the documented
  SplitMix64-to-ChaCha8 replay break on the frozen alpha RNG fixture.
  [Replay compatibility](../../release/beta-scope.md#alpha-replay-compatibility).
- **Timing and memory.** The installed SDK's serial scheduler drain had a
  median of 195.9 ms over 45 samples, with a range of 188.8–212.8 ms. The
  three peak process RSS values were 1,336,705,024, 938,115,072 and
  926,564,352 bytes. The conservative PDES ring preserved exact final-state
  parity across 120 matched pairs. Serial had lower median elapsed time
  in all eight measured cases.
  [Serial samples](evidence/serial-e287538.json) · [Paired samples](evidence/paired-pdes-e287538.json).
- **Core assurance.** The whole-core Linux campaign caught all 175 viable
  mutants with zero misses and timeouts. Seven scheduler property tests
  passed. The 60-second guided fuzz run executed 17,420,480 inputs and
  increased coverage from 66 to 127. AddressSanitizer and Miri execution
  remain unverified under the pinned profile.
  [Mutation run](https://github.com/edithatogo/kairos/actions/runs/45051175801) · [Property and fuzz run](https://github.com/edithatogo/kairos/actions/runs/45051181965).

Release Attestations verifies and signs the selected package digests using
the installation run and receipt hashes above.
[Provenance workflow](https://github.com/edithatogo/kairos/actions/workflows/release-attestations.yml).

## First native core candidate — 23 November 2026

The first `native-core-v1` candidate used main
`3b3b8da0a7cdbd414e12a18722f189996a52c7ab`. GitHub run
`45031314631` passed clean Rust SDK, C ABI v1 and CPython 3.15.2 installation
and execution on Linux x86-64 GNU and macOS ARM64. Both ran the installed
CareOps staged allocation case from
`6810aaaa38229b725a9fe0f99a0a6c1d8693ebd9`. The candidate is bound to the API
and targets in the [beta scope](../../release/beta-scope.md).
[Hosted run and exact archives](https://github.com/edithatogo/kairos/actions/runs/45031314631).

The shared Rust SDK SHA-256 is
`a8981c2bd9537dba20d6027d15c75e5deb329a9f0ee6912dab2d9609874006c9`.
Each target's C archive and Python wheel matched its own source-bound receipt.

| Target | Hosted installed receipt SHA-256 |
|---|---|
| Linux x86-64 GNU | `08608f185dcd5443a940262429213d407a8c3ba3ae20ed2dce4e1bf506c558fd` |
| macOS ARM64 | `cf5c1cf3f5ce009ab083d145cc5ae57c5bc9a66f113ff6e6a1fa694d04a2cc4f` |

An independent macOS install used the downloaded bytes, validated their
checksums and source fixture digests, and reran the C lifecycle, installed
Python lifecycle, Rust service-station and replay, and installed CareOps
allocation cases. Every invocation exited zero. The independent receipt
SHA-256 is `f71b182d0e78256da2c10cd029809872dffb0b789abce234ce38da5787504f2c`.

### First candidate compatibility, performance and provenance

- **CareOps replay.** The installed SDK built CareOps
  `9af99b5730d6aa5cef1c05bd48e90fd377b80cf2` on macOS ARM64 with Rust
  1.99.1. `c63_compatibility_replay_performance::historical_nominal_fixture_serializes_byte_identically`
  admitted the persisted scenario from provider pin `44261eb5cf8275f132f63a3396903d116d701dbe`
  and reproduced the frozen run byte for byte. The locked Cargo test exited
  zero. Test log SHA-256
  `fef95adbc919d2b58c558ffa6249129a2cd7c8abe835569f43afef0a1ac74cc5`.
- **Public alpha compatibility.** The published archive and tag identify
  `669520a292d4861cbe2971818100261cd62dedfc`. All 34 package manifest
  digests matched. The release body lists a different SHA. Both distributions
  passed the alpha ordering, cancellation and zero-delay fixtures. The alpha
  RNG fixture passed on the alpha and failed on the candidate as expected
  after the generator change. The candidate passed its own fixture. The
  locked Rust 1.99.1 comparison receipt SHA-256 is
  `4d8f04096551a1f4a845d6932f8ace9be3e3d4ee9a4d4ea447b38ab26bdd06ac`.
  [Published alpha archive](https://github.com/edithatogo/kairos/releases/tag/v0.4.0-alpha.1).
- **Timing and memory.** The installed SDK ran the existing serial
  `schedule_cancel_1m_mixed/schedule_cancel_pop` benchmark on Apple M1 Max
  with Rust 1.99.1 and seed 42. After scheduling one million events and
  cancelling half, 45 drain samples had a median of 196.5 ms and a range of
  187.7–209.7 ms. The three process RSS peaks were 926,466,048,
  978,944,000 and 1,026,064,384 bytes. RSS includes input setup. SHA-256
  `0200e4e4cdeb26161c8f493753d5ae314b3b57300da7ce8201b2cc60bffa1657`.
  [Raw serial distribution](evidence/serial-3b3b8da.json).
  The existing conservative PDES ring at the same source revision passed exact
  final-state parity across 120 matched serial and parallel samples. Serial
  was faster across the eight measured cases. SHA-256
  `a0d4e599f5f59ead98b677b0234991c716f76238597318c06c706b1dbcd156f3`.
  [Raw paired distribution](evidence/paired-pdes-3b3b8da.json).
- **Signed SBOM.** The approved installation run supplied six named artifact
  subjects with five unique digests. Verified Syft generated an SPDX 2.3
  inventory of 1,171 packages. The GitHub signature verified for all five
  digests against the release workflow on main
  `7c0c561ff2ee38431bc5b2d8e2d68a036996e4e5`. The SBOM SHA-256 is
  `13c34ae40c2b8094e7b80892b58e72a3c8a382344ad868516a99f6d9f2162275`.
  The verification bundle SHA-256 is
  `b47ca531eb685607e35ea994012825e1b118fbd3702ad77be628812f3e26b904`.
  [Hosted signature and evidence](https://github.com/edithatogo/kairos/actions/runs/45044299499).

The exact candidate's default scheduler and façade mutation run caught
56 viable mutants with zero misses and zero timeouts. The full Linux
mutation gate later passed on main at
`8332110ef2b4c5bed23c6064ca5c951779f25a10` with 175 caught and zero missed.
[Earlier scheduled baseline](https://github.com/edithatogo/kairos/actions/runs/45019966071).
[Passing whole-core run](https://github.com/edithatogo/kairos/actions/runs/45047387545).

The candidate also completed 20,000 coverage-guided scheduler inputs and a
60-second libFuzzer campaign with Rust 1.99.1. The long run executed
16,401,537 inputs and increased observed coverage from 66 to 127. The
retained macOS wheel rejected wrong architecture, missing symbols and
an incompatible ABI, then dispatched an event through the real C library.

This candidate's installation and signed SBOM are retained as prior
qualification evidence.

## Summary

Release a precisely scoped beta, then an independently reproducible and compatible stable core, without implying all experimental modules are complete.

## Files changed

This track's specification, plan, metadata, contract, risk register, test matrix and index; coordinated programme records and registries. Runtime changes and hosted outcomes are recorded separately in the integration receipt.

## Contracts consumed

AGENTS.md, existing core/FFI/conformance/versioning contracts, the approved [programme](../../frontier-programme-20261107.md), Tracks 15, 16, 20, 25, 109, 110, 111, 112, 114.

## Contracts changed

The programme establishes immediate Rust-only support; this track qualifies its own specific outcomes. No release/publication or downstream-pin acceptance follows from track registration.

## Tests added

Each implementation phase must add or identify the distinguishing oracles in test-matrix.md. Structural manifest/track checks are not native execution or scientific validation.

## Known risks

Stable branding can overgeneralize experimental scope. Version the supported manifest and keep experimental crates/features separately labelled; do not infer registry publication from packaging scripts.

## Follow-up issues

Execute the unchecked tasks in plan.md with original owner handoffs. Keep unsupported capability claims and release gates blocked until their evidence exists.

## Integration notes

Use isolated bounded worktrees. The root integration packet combines disjoint changes after review; actual commit/PR/CI outcomes go in the programme integration receipt. Preserve old immutable receipts.

## Phase closeout evidence

No phase is closed. Record $conductor-review, accepted fixes, commit SHA, pushed ref, scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree and the next-phase decision only when they occur. Current planning status is not execution evidence.


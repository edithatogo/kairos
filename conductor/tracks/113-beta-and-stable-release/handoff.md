# Handoff

## Native core v1 decision — 25 November 2026

The independent engineering review accepts the `native-core-v1` qualification
evidence for `e2875387ce45c34cd19716ea66f0102251a6aa6f`. Phase 0 is
pending maintainer acceptance of the [scope and replay change](../../release/beta-scope.md).
The selected profile uses Rust 1.99.1, C ABI v1 and CPython 3.15.2 on Linux
x86-64 GNU and macOS ARM64. ChaCha8 changes the seeded alpha replay draws.
The profiled core uses serial execution and preserves the documented boundary
for other bindings, platforms and experimental capabilities.

Integration chat `01a1fd18-51c7-7963-a3cc-d262d54582ba` records the Track 109
and 113 decisions from `/Users/doughnut/.codex/worktrees/kairos-native-beta-closeout/kairos`
on `codex/native-core-v1-acceptance-20261125`. The scope is both track records, the
phase ledger and status. Hosted access covers installation, policy and
provenance readback. The next operation is integration of the reviewed records.

### Exact artifacts

The two [installation receipts](metadata.json) match the retained archives
from [run 45051171243](https://github.com/edithatogo/kairos/actions/runs/45051171243).
The existing verifier matched all six archive paths to the receipts and to the
attestation's signed subject inventory. GitHub attestation verification passed for
all five distinct digests and the signed SPDX 2.3 document matched the retained
SBOM. [Attestation run](https://github.com/edithatogo/kairos/actions/runs/45053462090).

| Target | Package | SHA-256 |
| --- | --- | --- |
| Both | Rust SDK archive | `0060c934d7b5d9eb187387175fc4604184093823aedbfbb43fa52936b1a82240` |
| Linux x86-64 GNU | C ABI archive | `73efab28c21ff0651df3e14775f3a62fbdb11b6d0ef304d903dca29851fbf41e` |
| Linux x86-64 GNU | Python wheel | `7595df567c16dc5ced1bfdf866f93e552a5e8a187da2257d4e052512d3b10286` |
| macOS ARM64 | C ABI archive | `4e31fbfcf28cfb472970af6be803137152170b8be4ae839cbf4834eda3e5b6a9` |
| macOS ARM64 | Python wheel | `2ac30c0eb842de7f79be7beacfb6ad948f747dae396f2454f1a15316a3afe26f` |

### Gate readback

- The [Track 109 acceptance](../109-rust-only-runtime/handoff.md#native-runtime-acceptance--25-november-2026)
  binds retirement, dispatch, ABI errors and ownership to the scoped packages.
  Both installed targets ran the standalone Rust model, the C and Python
  lifecycle checks, and the pinned CareOps case. The candidate compatibility
  evidence below records analytical preemption, reproducible event ordering,
  replay, schema and buffer checks.
- Exact-source serial timing and memory samples are retained below. The matched
  PDES comparison also preserved model parity. Serial remains the profiled path.
- All 21 candidate push workflows passed. The exact [core mutation](https://github.com/edithatogo/kairos/actions/runs/45051175801)
  and [property/fuzz](https://github.com/edithatogo/kairos/actions/runs/45051181965)
  jobs passed. The hosted dependency check retained 14 policy-allowed advisory
  warnings, including transitive heapless. A fresh Rust 1.99.1
  `cargo deny --locked check advisories bans licenses sources` passed on
  the exact candidate source. [Hosted core check](https://github.com/edithatogo/kairos/actions/runs/45051171359).
- The selected SDK, C ABI and wheel passed clean install. The exact candidate
  also passed `build_release_manifest.py --check`, Rust types crate packaging
  and Python wheel/sdist build and Twine checks. Scope, compatibility
  guidance and experimental capability labels follow the
  [frozen manifest](../../../packaging/native-beta-scope.json) and
  [capability register](../../capabilities.md).

Stable qualification and registry publication follow the later release phases.

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
- **CareOps schema and buffer ownership.** The same downloaded SDK passed
  three R143 schema cases for populated ED, calibration and birthing
  projections. Two retained-buffer controls passed after producer and
  stream release. All nine resolved Kairos crates came from the installed SDK.
  Test log SHA-256
  `b54ea4b32fef67ed4e192cee977667f070ff0d6049951ea91a90756c04cbc19b`.
- **Resource preemption.** The installed SDK matched an independently
  calculated 15-row trace for priority replacement, equal and lower priority
  queueing, full-width interruption ticks, remaining work, capacity and
  priority/FIFO release.
  [Resource trace](evidence/preemption-e287538.json).
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

The `$conductor-review` evidence and accepted fixes are recorded above.
The [phase ledger](../../phase-closeout.yaml) binds the candidate commit SHA
to the pushed ref and records the next-phase decision. Integration of this
record uses `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.


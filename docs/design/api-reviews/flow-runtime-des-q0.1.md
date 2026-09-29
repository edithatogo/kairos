# API design disposition: DES FlowRuntime root (Q0.1)

This is a preimplementation design disposition using the Track 25 review fields.
It is **not** a completed symbol-level Track 25 API review and does not authorize
release.

## Intake

| Field | Disposition |
|---|---|
| Review title | Additive experimental FlowRuntime in DES |
| Proposed by | CareOps Sim coordinator under Kairos owner direction |
| Affected root | `crates/kairo-ecs-des` |
| Surface family | `rust_api` |
| Current status | `experimental` |
| Proposed release stage | `alpha` candidate, not authorized |
| Compatibility level | `experimental-breaking` for the new surface |
| Decision | Accept architecture/root classification for design; hold concrete symbols and release |

## Proposed change and compatibility

The existing root already exists in the workspace and is now listed exactly in
`docs/design/protected-surface-inventory.json`. The design adds a separate
Rust-native `FlowRuntime`, opaque resource/claim/work/lease identities, queue
commands, deterministic dispatch, and preemption lifecycle support. It preserves
the existing `DESContext` and FIFO `Resource` API and behavior.

| Review question | Answer |
|---|---|
| Additive only? | Yes, if legacy DES behavior remains unchanged. |
| Existing public semantics changed? | No change is intended; verify against implementation tests. |
| Root renamed/split/merged/removed? | No. |
| Scheduler order, replay, or RNG changed? | No; preserve core order and RNG derivation. |
| C ABI or Arrow schema changed? | No. |
| Host binding changed? | No. |
| Consumer migration required? | None for existing DES users. |

## Evidence and release gate

- Architecture record: parent CareOps ADR-0003.
- Runtime boundaries: `conductor/research/careops-flow-runtime-contract-proposal-20260929.md`.
- Inventory/policy/matrix/release note: protected-surface inventory, versioning
  contract, compatibility matrix, and `docs/release/compatibility.md`.
- Track 01 review confirmed the lifetime overflow envelope only for a fresh
  private runtime with facade-only mutation. Cleanup and reverse-reference
  handling remain implementation invariants.

**Release hold:** yes. Exact public symbols, error variants, builder signatures,
and method semantics remain unreviewed until Q0.2/Q0.3 freeze them. Complete a
symbol-level review and compatibility tests before release; alpha is not
authorized by this design disposition.

## Reviewer and authority boundary

- Owner direction: Kairos repository owner approved the Q0.1 architecture and
  experimental root classification on 2026-09-29.
- Technical review: independent read-only Track 01 review; not a release signoff.
- API governance reviewer for concrete symbols: pending implementation review.
- Release reviewer: pending; release remains held.
- Red-team review: not performed.

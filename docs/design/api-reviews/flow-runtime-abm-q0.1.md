# API design disposition: ABM Flow adapter root (Q0.1)

This is a preimplementation design disposition using the Track 25 review fields.
It is **not** a completed symbol-level Track 25 API review and does not authorize
release.

## Intake

| Field | Disposition |
|---|---|
| Review title | Shared-runtime Flow behavior adapter in ABM |
| Proposed by | CareOps Sim coordinator under Kairos owner direction |
| Affected root | `crates/kairo-ecs-abm` |
| Surface family | `rust_api` |
| Current status | `experimental` |
| Proposed release stage | `alpha` candidate, not authorized |
| Compatibility level | `experimental-breaking` for the new adapter surface |
| Decision | Accept architecture/root classification for design; hold concrete symbols and release |

## Proposed change and compatibility

The existing root already exists in the workspace and is now listed exactly in
`docs/design/protected-surface-inventory.json`. The design adds a Flow-specific
ABM adapter that depends on DES and operates on the DES-owned shared scheduler,
world, and component registry. DES owns dispatch and handler registration; the
ABM adapter supplies agent policy through restricted typed queries and buffered
checked commands. No DES-to-ABM dependency is introduced.

| Review question | Answer |
|---|---|
| Additive only? | Yes, if legacy ABM behavior remains unchanged. |
| Existing public semantics changed? | No change is intended; verify against implementation tests. |
| Root renamed/split/merged/removed? | No. |
| Scheduler order, replay, or RNG changed? | No; preserve core order and current RNG derivation. |
| C ABI or Arrow schema changed? | No. |
| Host binding changed? | No. |
| Consumer migration required? | None for existing ABM users. |

## Evidence and release gate

- Architecture record: parent CareOps ADR-0003.
- Runtime and adapter boundary:
  `conductor/research/careops-flow-runtime-contract-proposal-20260929.md`.
- Inventory/policy/matrix/release note: protected-surface inventory, versioning
  contract, compatibility matrix, and `docs/release/compatibility.md`.
- Track 03 review confirmed that separate legacy `ABMContext` and
  `BehaviorSimulation` cannot provide the shared Flow runtime, and that the
  ABM-to-DES workspace dependency is acyclic.

**Release hold:** yes. Exact callback/context/adapter signatures, per-agent RNG
stream lifecycle, query guarantees, buffered-command validation, and event joins
remain unreviewed until Q0.2/Q0.3 freeze them. Complete a symbol-level review and
compatibility tests before release; alpha is not authorized by this design
disposition.

## Reviewer and authority boundary

- Owner direction: Kairos repository owner approved the Q0.1 architecture and
  experimental root classification on 2026-09-29.
- Technical review: independent read-only Track 03 review; not a release signoff.
- API governance reviewer for concrete symbols: pending implementation review.
- Release reviewer: pending; release remains held.
- Red-team review: not performed.

# API Review: CareOps staged resource runtime

## Affected surface

- Affected root: `crates/kairo-ecs-des`
- Surface family: `rust_api`
- Current status: experimental
- Release stage: alpha
- Related track: 107

## Proposed change

- Summary: Add purpose-specific experimental `StagedResourceRuntime` for scheduler-backed staged allocation.
- Consumer-visible behavior: New API only. CareOps source migrates from the removed `FlowRuntime`; no former broad API compatibility is claimed.
- Protected root inventory: Added as `rust-des-api` in `docs/design/protected-surface-inventory.json`.

## Compatibility classification

| Question | Answer |
|---|---|
| Additive only? | yes |
| Alters public semantics? | no existing semantics; defines new experimental semantics |
| Renames/splits/removes a protected root? | no |
| Alters ordering/replay/fixture output? | no scheduler ordering change; adds versioned fixture |
| Requires source edits/adapters/pins? | yes, CareOps migration from removed API |

## Required evidence

- ADR: `docs/adr/adr-track107-careops-staged-flow.md`
- Migration: `docs/release/migration.md`
- Fixture/test: `conformance/fixtures/track107-careops-staged-allocation.json` and crate conformance test
- API governance: inventory, compatibility matrix, versioning contract and release compatibility note updated
- Reviewer-agent API governance and red-team review: completed for source commit `5897ebf95d5993d069d85a1fd70a1a62123c0873`; see `evidence/reviewer-agent-review.md`

## Release decision

- Compatibility level: `compatible` additive experimental API at alpha
- Release hold: yes for beta or later until CareOps migration, conformance and seeded replay gates pass
- First release stage allowed: alpha
- Transition: migrate CareOps consumer; do not recreate the removed `FlowRuntime` surface

## Reviewer signoff

- API governance reviewer: two independent reviewer agents assessed source commit `5897ebf95d5993d069d85a1fd70a1a62123c0873`; no remaining code or contract blockers
- Release reviewer: CareOps migration and seeded replay remain explicit downstream acceptance gates
- Red-team objection status: no remaining blocker within the reviewed staged-allocation scope; full former `FlowRuntime` compatibility remains explicitly excluded
- Final decision: accepted for alpha implementation; beta-or-later remains on hold pending CareOps migration/acceptance

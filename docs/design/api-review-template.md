# API Review Template

Use this template for any change that touches a protected surface in
`docs/design/protected-surface-inventory.json`.

## Intake

| Field | Required value |
|---|---|
| Review title | `<short name>` |
| Proposed by | `<person or track>` |
| Affected root | One exact root from the protected-surface inventory |
| Surface family | `rust_api`, `c_abi`, `arrow_schema`, `host_api`, or `conformance` |
| Release stage | `alpha`, `beta`, `rc`, or `1.0` |
| Compatibility level | `compatible`, `experimental-breaking`, `breaking`, or `release-hold` |
| Decision | `accepted`, `rejected`, `needs ADR`, `needs migration note`, or `release hold` |

## Review Form

```markdown
# API Review: <short name>

## Affected Surface

- Affected root:
- Surface family:
- Current status:
- Release stage:
- Related track or PR:

## Proposed Change

- Summary:
- Public API, ABI, schema, fixture, or host behavior changed:
- New root, renamed root, split root, or removed root:
- Consumer-visible behavior:

## Compatibility Classification

| Question | Answer |
|---|---|
| Is the change additive only? | yes/no |
| Does it alter public semantics? | yes/no |
| Does it rename, split, merge, or remove a protected root? | yes/no |
| Does it alter deterministic ordering, replay, or fixture output? | yes/no |
| Does it alter C ABI ownership, allocation, status, or symbol shape? | yes/no |
| Does it remove, retype, rename, or change meaning for an Arrow field? | yes/no |
| Does downstream code need source edits, adapters, or version pins? | yes/no |

## Required Evidence

- Compatibility note path:
- ADR path, or reason not required:
- Migration note path, or reason not required:
- Release note path:
- Tests, fixtures, or schema checks:
- Package catalog or matrix update, or reason not required:

## Release Decision

- Compatibility level:
- Release hold: yes/no
- Release-hold reason:
- First release stage allowed:
- Deprecation or transition plan:

## Reviewer Signoff

- API governance reviewer:
- Release reviewer:
- Red-team objection status:
- Final decision:
```

## Blocking Rules

- The affected root must exactly match a root in
  `docs/design/protected-surface-inventory.json`.
- Any `breaking` change requires an ADR and a migration note before beta, RC,
  or 1.0.
- Any renamed, split, merged, or removed protected root is a release hold until
  the transition is documented.
- Any review that cannot classify the compatibility level is incomplete.

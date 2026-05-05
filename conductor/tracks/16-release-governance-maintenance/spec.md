# 16 Release Governance & Maintenance — spec.md

## Mission

Define versioning, compatibility, deprecation, maintenance, security response, support windows, and release trains.

## Primary subagent

```text
release-agent + security-agent
```

## Dependencies

```text
Track 00. Starts immediately.
```

## Owned paths

```text
CHANGELOG, docs/release, governance docs
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- `conductor/contracts/naming-contract.md` and approved project identity.
- `CHANGELOG.md`, `SECURITY.md`, `CODEOWNERS` (current state).
- `conductor/package-matrix.md` (ecosystem list that must be covered).
- Handoff notes from Tracks 15 (packaging plan) and 20 (supply-chain gates).
- Prior release tag history and semver baseline.

## Outputs

- `CHANGELOG.md`: populated with release entries following Keep a Changelog conventions.
- `docs/release/release-governance.md`: versioning, compatibility, deprecation, and support-window policy.
- `docs/release/release-checklist.md`: step-by-step release procedure per ecosystem.
- `docs/release/maintainer-rotation.md`: polyglot coverage map and escalation path.
- `conductor/tracks/16-release-governance-maintenance/test-matrix.md`: CI gate definitions (changelog lint, semver check, checklist automation).




## Acceptance criteria

- Owned paths are created and documented.
- Contract inputs and outputs are explicit.
- Track tests or validation checks exist.
- CI gate is defined.
- Documentation impact is recorded.
- Release implications are recorded.
- `handoff.md` is completed before merge.


## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.




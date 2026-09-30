# 44 Code and Repository Health >= 9.5 - spec.md

## Mission

Make `>= 9.5/10` code and repository health a hard gate for production publication.

Track 44 defines a weighted scorecard and machine-checkable baseline that registry publication tracks consume before any public write.

## Primary subagent

```text
health-agent + ci-agent + security-agent + release-agent
```

## Dependencies

```text
Tracks 13, 20, 25, 28, 30, 31, and 41.
```

## Owned paths

```text
conductor/code-health.md
.github/workflows/code-health.yml
scripts/validation/validate-code-health.mjs
conductor/tracks/44-code-repository-health-95/*
```

## Acceptance criteria

- Code/repo health score target is documented as `>= 9.5`.
- Validator enforces the health floor and required evidence surfaces.
- CI workflow runs the validator.
- Track 42 and Track 43 consume the health gate before publication.
- Any waiver must be explicit, time-bound, and release-manager approved.

## Release implications

Track 44 is release-gating for production registry publication, beta, RC, 1.0, and production-ready cloud/HPC claims. A score below 9.5 blocks those release stages.

## Blocked paths

Lowering or waiving the 9.5 health floor is blocked unless release governance records an explicit, time-bound waiver with release-manager approval.

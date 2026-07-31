# 42 Package Registry Publication & Provenance - spec.md

## Mission

Implement the production-ready publication lane for every language/package registry without allowing accidental public writes.

Track 42 turns the Track 15 dry-run inventory into guarded registry publication workflows for Rust, Python, R, Julia, TypeScript, C#, and Go. The implementation must prefer trusted publishing, OIDC, provenance, and protected GitHub environments where the target registry supports them.

## Primary subagent

```text
publication-agent + release-agent + binding agents
```

## Dependencies

```text
Tracks 06-11, 15, 16, 20, 25, 28, 29, 30, and 44.
```

## Owned paths

```text
.github/workflows/registry-publish.yml
packaging/publication-registry-manifest.json
scripts/validation/validate-publication-readiness.mjs
conductor/tracks/42-package-registry-publication-provenance/*
```

## Acceptance criteria

- A manifest lists every language/package registry lane.
- A workflow supports dry-run validation for every lane.
- Public publish mode is manually dispatched, protected-environment gated, and defaults off.
- Supported registries prefer trusted publishing/OIDC and provenance.
- Token fallback lanes are documented as lower-trust, protected, and rotation-bound.
- Every package lane records SOTA controls: SBOM/checksums, provenance/attestation, conformance, docs link, compatibility notes, owner, and rollback/yank plan.
- Publication is blocked unless Track 44 reports code/repo health `>= 9.5`.

## Release implications

Track 42 is release-gating for all public language registry writes. It does not itself make Track 32-35 or Track 39 runtime evidence complete.

## Blocked paths

Public registry writes remain blocked until registry-side trusted publisher/account setup, protected environment approval, Track 44 health evidence, and release-manager signoff are recorded.

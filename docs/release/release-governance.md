# Release Governance

This page is the Track 16 release-governance policy for R2. It describes the
minimum controls that must pass before a KairoECS release can move from dry-run
evidence to publication. Registry writes remain blocked until Track 15 package
handoff and naming evidence are complete.

The enforcement path is intentionally simple:

1. `docs/release/changelog-policy.md` defines the public-surface changelog gate.
2. `conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1`
   checks that the release docs still describe the gate and the governance
   boundary.
3. `docs/release/release-checklist.md` is the release-manager checklist of
   record.
4. `docs/release/maintenance-handoff.md` records the evidence and remaining
   blockers before any public write.

## Release posture

- R2 governance is evidence-first: release candidates may produce manifests,
  checksums, SBOM/provenance evidence, dry-run packages, and release notes.
- Production publication is blocked until naming/package availability, registry
  credentials, compatibility review, and release evidence are all complete.
- Public release-surface changes must carry a changelog entry before the gate
  can clear.
- The release manager must use `docs/release/release-checklist.md` as the
  checklist of record.
- Release notes must be updated in `docs/release/release-notes.md` before a
  public tag or archive record is prepared.
- The PR-level changelog gate is implemented in
  `.github/workflows/changelog-policy.yml` and mirrors the local changelog
  policy check.

## Versioning rule

Use semver for user-facing package versions.

- Patch: compatible fixes, docs clarifications, and non-breaking CI/release
  repairs.
- Minor: additive public API, ABI, schema, fixture, or package-surface changes.
- Major: breaking API, ABI, schema, fixture, or package-root changes after 1.0.
- Pre-1.0 breaking changes still require a compatibility note, changelog entry,
  and migration/deprecation evidence when users could reasonably depend on the
  surface.

## Compatibility gate

The compatibility boundary is the inventory in
`conductor/contracts/versioning-compatibility.md`.

A release is held when any changed public surface lacks:

- a named affected root;
- a changelog entry;
- a compatibility note in `docs/release/compatibility.md`;
- an ADR when the change is breaking or changes a compatibility promise;
- a migration note in `docs/release/migration.md` when users must change code,
  fixtures, package names, or deployment scripts.

The changelog gate is part of the same enforcement path. If a public surface
changes, the release manager must see the matching `CHANGELOG.md` entry before
the compatibility gate can be treated as satisfied.

## Deprecation gate

Deprecations must be visible before removals.

- Add the deprecation to `CHANGELOG.md` under `Deprecated`.
- Add the affected root and replacement path to `docs/release/compatibility.md`.
- Add runtime/compiler warnings where the implementation surface can support
  them.
- Keep at least one minor release of notice before removal where feasible
  before 1.0.
- After 1.0, removals require a major version unless an ADR documents a
  compatible migration.

## Evidence gate

The release manager records release evidence before publish:

- checklist status from `docs/release/release-checklist.md`;
- dry-run package evidence from Track 15 outputs;
- changelog policy workflow status from `.github/workflows/changelog-policy.yml`;
- changelog and release-note links;
- compatibility/deprecation status;
- manifest and checksum paths when artifacts are generated;
- SBOM/provenance/attestation workflow status when present;
- unresolved blockers and the owner for each blocker.

The handoff record belongs in `docs/release/maintenance-handoff.md`.

This evidence gate is what the release-governance path uses to prove that the
changelog check, compatibility review, package dry-run, and attestation status
were all observed before publication.

# Release Governance

This page is the Track 16 release-governance policy for R2. It describes the
minimum controls that must pass before a KairoECS release can move from dry-run
evidence to publication. Registry writes remain blocked until Track 15 package
handoff and naming evidence are complete.

## Release posture

- R2 governance is evidence-first: release candidates may produce manifests,
  checksums, SBOM/provenance evidence, dry-run packages, and release notes.
- Production publication is blocked until naming/package availability, registry
  credentials, compatibility review, and release evidence are all complete.
- The release manager must use `docs/release/release-checklist.md` as the
  checklist of record.
- Release notes must be updated in `docs/release/release-notes.md` before a
  public tag or archive record is prepared.

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
- changelog and release-note links;
- compatibility/deprecation status;
- manifest and checksum paths when artifacts are generated;
- SBOM/provenance/attestation workflow status when present;
- unresolved blockers and the owner for each blocker.

The handoff record belongs in `docs/release/maintenance-handoff.md`.

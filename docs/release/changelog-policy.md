# Changelog Policy and Check

`CHANGELOG.md` is the source of user-facing release history. Release notes may
expand on it, but they must not introduce compatibility claims that are absent
from the changelog or compatibility note.

The implemented CI gate for this policy is `.github/workflows/changelog-policy.yml`.
It blocks pull requests that touch public release surfaces without also updating
`CHANGELOG.md`.

The implemented changelog gate is track-local and release-governance aware:
Track 16 documents the rule, the release manager runs the local governance
validator, and any public release surface change is blocked until the changelog
is updated.

## Required entry

A PR must update `CHANGELOG.md` when it changes any of these public surfaces:

- `crates/kairo-ecs-types`
- `crates/kairo-ecs-core`
- `crates/kairo-ecs-state`
- `crates/kairo-ecs-rng`
- `crates/kairo-ecs-ffi`
- `include/`
- `bindings/python`
- `bindings/r`
- `bindings/julia`
- `bindings/typescript`
- `bindings/csharp`
- `bindings/go`
- `schemas/arrow/`
- `conformance/fixtures/`
- `docs/release/`
- release workflows that create manifests, checksums, SBOMs, provenance, or
  registry dry-runs

## Entry format

- Keep `## Unreleased` at the top.
- Use `Added`, `Changed`, `Deprecated`, `Removed`, and `Fixed`.
- Name the affected root when the change affects compatibility.
- Link release notes or archive metadata when a release is archived or
  DOI-minted.

## Static check

The release manager can also run this local check before handoff:

```powershell
$changed = git diff --name-only HEAD
$public = $changed | Where-Object {
    $_ -match '^(crates/kairo-ecs-(types|core|state|rng|ffi)|include/|bindings/(python|r|julia|typescript|csharp|go)|schemas/arrow/|conformance/fixtures/|docs/release/|\.github/workflows/)'
}
if ($public -and ($changed -notcontains 'CHANGELOG.md')) {
    throw 'Public release surface changed without CHANGELOG.md'
}
```

The same rule is enforced in the Track 16 governance path: the release
checklist, the changelog policy, and the release-governance handoff all point
to the same requirement that a public release surface change must carry a
matching changelog entry before publish can proceed.

CI should mirror the same rule against the PR diff before any public release
workflow can publish.

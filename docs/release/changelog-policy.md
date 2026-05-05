# Changelog Policy and Check

`CHANGELOG.md` is the source of user-facing release history. Release notes may
expand on it, but they must not introduce compatibility claims that are absent
from the changelog or compatibility note.

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

Until a dedicated workflow is added, the release manager can run this local
check before handoff:

```powershell
$changed = git diff --name-only HEAD
$public = $changed | Where-Object {
    $_ -match '^(crates/kairo-ecs-(types|core|state|rng|ffi)|include/|bindings/(python|r|julia|typescript|csharp|go)|schemas/arrow/|conformance/fixtures/|docs/release/|\.github/workflows/)'
}
if ($public -and ($changed -notcontains 'CHANGELOG.md')) {
    throw 'Public release surface changed without CHANGELOG.md'
}
```

CI should implement the same rule against the PR diff before any public release
workflow can publish.

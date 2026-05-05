# Agent Contract: toolchain-agent

## Track

Track 30: Toolchain & Version Support Matrix

## Owned paths

- `conductor/tracks/30-toolchain-version-support-matrix/`
- `conductor/toolchain-matrix.md`
- `.github/workflows/toolchain-check.yml`
- Track-specific artifacts named in `plan.md`

## Required handoff

- The completed support matrix with all language rows.
- The version-drop policy.
- CI verification workflow status.
- Gate definitions and pass/fail semantics.
- List of binding tracks that lack explicit version declarations.
- Follow-up items for CI/CD (Track 13) and each binding track (06-11).

## Prohibited changes without ADR

- Changing minimum supported versions for any language without updating the matrix and notifying binding track owners.
- Modifying binding package manifests (`Cargo.toml`, `pyproject.toml`, `Package.swift`, etc.).
- Modifying `rust-toolchain.toml` (owned by Track 13).
- Modifying existing CI workflows not named `toolchain-check.yml`.
- Dropping a version from the matrix without following the version-drop policy.

## Gate contract

### toolchain-matrix-current
- **Input**: `conductor/toolchain-matrix.md`, live toolchain versions on CI runners.
- **Output**: Pass if every declared supported version is verified on at least one CI runner. Fail with a list of matrix rows that have no CI coverage.
- **Blocking**: Yes — prevents release if declared support is unverifiable.

### version-drop-policy-check
- **Input**: `conductor/toolchain-matrix.md` diff, release notes.
- **Output**: Pass if no version has been removed without prior deprecation notice and the required notice period. Fail with the dropped version and the missing notice detail.
- **Blocking**: Yes — prevents release if a version was dropped without following policy.

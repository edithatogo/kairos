# Track 30 Plan: Toolchain & Version Support Matrix

## Phase 0 - Scope lock

### Task 0.1 - Inventory current toolchain state
- Read `rust-toolchain.toml` for the current Rust version.
- Inspect `.github/workflows/` for CI runner OS/arch coverage.
- Survey each binding track (06-11) for declared or implied version requirements.
- Document gaps where a binding has no explicit minimum version.

### Task 0.2 - Lock the owned surface
- Keep new work to `conductor/tracks/30-toolchain-version-support-matrix/`.
- Write `conductor/toolchain-matrix.md` and `.github/workflows/toolchain-check.yml` only.
- Do not modify binding source files, package manifests, or existing CI workflows.

## Phase 1 - Build the support matrix

### Task 1.1 - Define supported versions per language
- Rust: stable, beta, and MSRV per `rust-toolchain.toml`.
- Python: 3.10, 3.11, 3.12, 3.13, 3.14 (free-threaded where applicable).
- .NET: 10.0 (stable), 11.0 (preview).
- Julia: LTS and current stable.
- R: current release and previous release.
- Go: two most recent stable releases.
- Node/TypeScript: LTS and current.

### Task 1.2 - Define OS/arch support
- Per language, specify: Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64, Windows x86_64.
- Mark any OS/arch as "best-effort", "CI-covered", or "unsupported".

### Task 1.3 - Publish the matrix
- Write `conductor/toolchain-matrix.md` as a markdown table.
- Include deprecation horizon column.

## Phase 2 - Define version-drop policy

### Task 2.1 - Specify drop rules
- Minimum notice period: 2 release cycles or 6 months, whichever is longer.
- Deprecation signalling: a warning in release notes and a deprecation notice in the binding's README.
- Removal criteria: when the upstream vendor ends support OR when CI coverage for that version becomes unsustainable.

### Task 2.2 - Encode policy
- Write the version-drop policy into `conductor/toolchain-matrix.md`.
- Add `version-drop-policy-check` gate definition.

## Phase 3 - CI verification workflow

### Task 3.1 - Write toolchain-check.yml
- Read the matrix from `conductor/toolchain-matrix.md`.
- For each binding with CI coverage, verify the installed toolchain version matches the declared support range.
- Fail with a specific version mismatch message.

### Task 3.2 - Wire the gates
- Add `toolchain-matrix-current` and `version-drop-policy-check` to `conductor/quality-gates.md`.
- Ensure the workflow is triggered on PRs that modify binding package manifests.

## Phase 4 - Handoff and closeout

### Task 4.1 - Prepare maintainer notes
- Document how to update the matrix when a new language version is released.
- Document how to propose dropping a version.
- List which CI runners are at risk of falling out of support.

### Task 4.2 - Cross-track communication
- Hand off matrix to each binding track (06-11) as their authoritative version floor.
- Hand off to Track 13 (CI/CD) for runner provisioning alignment.
- Hand off to Track 15 (Packaging) for release checklist integration.

### Task 4.3 - Update the risk register
- Mark resolved risks as mitigated.
- Escalate unresolved toolchain gaps to release blockers.

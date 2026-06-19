# Track 30 Plan: Toolchain & Version Support Matrix

## Phase 0 - Scope lock

### Task 0.1 - Inventory current toolchain state [x]
- Read `rust-toolchain.toml` for the current Rust version.
- Inspect `.github/workflows/` for CI runner OS/arch coverage.
- Survey each binding track (06-11) for declared or implied version requirements.
- Document gaps where a binding has no explicit minimum version.

Evidence 2026-05-06: read `rust-toolchain.toml`, `Cargo.toml`, `mise.toml`, `.github/workflows/ci-bindings.yml`, `.github/workflows/package-dry-run.yml`, and binding manifests for Tracks 06-11. Gaps documented in `conductor/toolchain-matrix.md`: cross-OS/arch coverage is not provisioned yet. The TypeScript manifest now declares `engines.node = ">=22 <25"`, and the package dry-run Go lane now uses supported floor `1.25.x`.

### Task 0.2 - Lock the owned surface [x]
- Keep new work to `conductor/tracks/30-toolchain-version-support-matrix/`.
- Write `conductor/toolchain-matrix.md` and `.github/workflows/toolchain-check.yml` only.
- Do not modify binding source files, package manifests, or existing CI workflows.

Evidence 2026-05-06: this slice changed only Track 30 artifacts, `conductor/toolchain-matrix.md`, `.github/workflows/toolchain-check.yml`, and the Track 30 rows in `conductor/quality-gates.md`.

## Phase 1 - Build the support matrix

### Task 1.1 - Define supported versions per language [x]
- Rust: stable, beta, and MSRV per `rust-toolchain.toml`.
- Python: 3.10, 3.11, 3.12, 3.13, 3.14 (free-threaded where applicable).
- .NET: 10.0 (stable), 11.0 (preview).
- Julia: LTS and current stable.
- R: current release and previous release.
- Go: two most recent stable releases.
- Node/TypeScript: LTS and current.

Evidence 2026-05-06: matrix now records Rust MSRV 1.76/stable 1.95/beta lane, Python 3.10-3.14, R 4.2 package floor plus 4.5/4.6 CI lanes, Julia 1.10/1.12, Node 22/24 with Node 20 deprecated, .NET 10/11, and Go 1.25/1.26 with Go module floor 1.23. Evidence 2026-06-19: GitHub Actions current-stable Rust refreshed to 1.96.x after hosted Linux stable reported rustc 1.96.0.

### Task 1.2 - Define OS/arch support [x]
- Per language, specify: Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64, Windows x86_64.
- Mark any OS/arch as "best-effort", "CI-covered", or "unsupported".

### Task 1.3 - Publish the matrix [x]
- Write `conductor/toolchain-matrix.md` as a markdown table.
- Include deprecation horizon column.

## Phase 2 - Define version-drop policy

### Task 2.1 - Specify drop rules [x]
- Minimum notice period: 2 release cycles or 6 months, whichever is longer.
- Deprecation signalling: a warning in release notes and a deprecation notice in the binding's README.
- Removal criteria: when the upstream vendor ends support OR when CI coverage for that version becomes unsustainable.

### Task 2.2 - Encode policy [x]
- Write the version-drop policy into `conductor/toolchain-matrix.md`.
- Add `version-drop-policy-check` gate definition.

## Phase 3 - CI verification workflow

### Task 3.1 - Write toolchain-check.yml [x]
- Read the matrix from `conductor/toolchain-matrix.md`.
- For each binding with CI coverage, verify the installed toolchain version matches the declared support range.
- Fail with a specific version mismatch message.

Evidence 2026-05-06: `.github/workflows/toolchain-check.yml` runs static policy validation plus installed version checks for Rust stable/beta, Python 3.10-3.14, R oldrel-1/release, Julia 1.10/1.12, Node 22/24, .NET 10/11, and Go 1.25/1.26.

### Task 3.2 - Wire the gates [x]
- Add `toolchain-matrix-current` and `version-drop-policy-check` to `conductor/quality-gates.md`.
- Ensure the workflow is triggered on PRs that modify binding package manifests.

## Phase 4 - Handoff and closeout

### Task 4.1 - Prepare maintainer notes [x]
- Document how to update the matrix when a new language version is released.
- Document how to propose dropping a version.
- List which CI runners are at risk of falling out of support.

### Task 4.2 - Cross-track communication [x]
- Hand off matrix to each binding track (06-11) as their authoritative version floor.
- Hand off to Track 13 (CI/CD) for runner provisioning alignment.
- Hand off to Track 15 (Packaging) for release checklist integration.

### Task 4.3 - Update the risk register [x]
- Mark resolved risks as mitigated.
- Escalate unresolved toolchain gaps to release blockers.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update the track registry/status surfaces: `conductor/tracks.yaml` (authoritative machine-readable registry), `conductor/tracks.md` (human index), `conductor/phase-closeout.yaml` (review ledger), `conductor/status.md` (narrative status), and `conductor/implementation-readiness.md` or `conductor/track-map.md` when readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.

# Handoff: Track 30 Toolchain & Version Support Matrix

## Summary

Current slice completed on 2026-05-07. Defined the cross-language toolchain version support matrix. Track 30 now has a current toolchain matrix, explicit version-drop policy, workflow-backed gate validation, and a local PowerShell validator. No binding manifests or source files were modified.

2026-05-10 review update: focused review found no blocking defects in the
matrix, version-drop policy, workflow trigger, or validator slice. The static
Track 30 validator passed again, so the track is ready for integration review.

2026-05-10 closeout update: PR #18 merged to `origin/main` as
`98f04cae89fa434a8dcca05d9022db090561c7c7`, all GitHub Actions checks for that
merge passed, branch protection was restored, and the post-merge focused
validator passed again. Track 30 is closed as `Done`.

## Files changed

- `conductor/toolchain-matrix.md`
- `.github/workflows/toolchain-check.yml`
- `conductor/quality-gates.md`
- `conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1`
- `conductor/tracks/30-toolchain-version-support-matrix/plan.md`
- `conductor/tracks/30-toolchain-version-support-matrix/test-matrix.md`
- `conductor/tracks/30-toolchain-version-support-matrix/risk-register.md`
- `conductor/tracks/30-toolchain-version-support-matrix/handoff.md`

## Evidence read

- `rust-toolchain.toml`: Rust channel is `stable`.
- `Cargo.toml`: workspace `rust-version` is `1.76`.
- `mise.toml`: developer defaults are Rust stable, Python 3.14, Node LTS, Go latest, Julia latest, R latest, .NET 10.0.
- `bindings/python/pyproject.toml`: Python floor is `>=3.10`.
- `bindings/r/DESCRIPTION`: R floor is `R (>= 4.2)`.
- `bindings/julia/Project.toml`: Julia compat floor is `1.10`.
- `bindings/typescript/package.json`: declares `engines.node = ">=22 <25"`.
- `bindings/csharp/global.json`: SDK default is `10.0.202`.
- `bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj`: target frameworks are `net10.0;net11.0`.
- `bindings/go/go.mod`: module floor is `go 1.23`.
- `.github/workflows/ci-bindings.yml`: existing binding CI is Ubuntu-hosted and covers Python 3.10-3.14, Node LTS, .NET 10/11, and Go stable.
- `.github/workflows/package-dry-run.yml`: package dry-run now references Go `1.25.x`, matching the supported CI floor.

## Contracts consumed

- `conductor/track-map.md`: binding track language list.
- `conductor/tracks.yaml`: track inventory and dependency structure.
- `conductor/workflow.md`: owned path and handoff expectations.
- `rust-toolchain.toml`, `Cargo.toml`, `mise.toml`, and binding manifests as read-only evidence.

## Release gates affected

- **toolchain-matrix-current**: Added as a Track 30 quality gate. The gate runs static policy validation and the GitHub Actions workflow installs declared CI selectors for Rust, Python, R, Julia, Node, .NET, and Go.
- **version-drop-policy-check**: Added as a Track 30 quality gate. The current gate enforces policy structure, proposed-drop records, release-note/README signal, removal criteria, waiver path, and workflow lane consistency. Historical previous-matrix diff enforcement remains a follow-up input once release baselines are archived.

## Validation commands

| Command | Result | Notes |
|---|---|---|
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1` | Pass | Static policy/gate/workflow validation passed locally on 2026-05-07. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem rust -ExpectedPrefix 1.94` | Pass | Local Rust reports 1.94. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem rust -ExpectedPrefix 1.95` | Expected fail | Local Rust is behind the matrix current-stable lane; workflow installs stable explicitly. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 3.13` | Pass | Local installed Python reports 3.13 and matches expected prefix. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem node -ExpectedPrefix 24` | Pass | Local Node reports 24. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem dotnet -ExpectedPrefix 11.0` | Pass | Local .NET reports 11.0. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem dotnet -ExpectedPrefix 10.0` | Expected fail | Local .NET defaults to 11.0; workflow installs 10.0 and 11.0 explicitly. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem go -ExpectedPrefix 1.26` | Pass | Local Go reports 1.26. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 3.11` | Expected fail | Local installed Python is 3.13; validator failed with `python version mismatch`. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 9.99` | Expected fail | Negative proof that mismatch detection fails closed. |

## Handoff notes

- Track 06: Python support remains 3.10-3.14; free-threaded Python 3.14 is advisory until a runner can prove it.
- Track 07: R package floor remains 4.2, but Track 30 CI covers oldrel-1/current release.
- Track 08: Julia support remains 1.10 LTS-compatible through 1.12 current stable.
- Track 09: Keep `engines.node` aligned with Node 22/24 production CI support floors and the version-drop policy.
- Track 10: .NET 10 is stable; .NET 11 remains preview/experimental until GA.
- Track 11: Go module floor remains 1.23; CI support floor is 1.25/1.26.
- Track 13: Provision non-Ubuntu and aarch64 runner coverage before RC if those cells should become release-supported.
- Track 15: Keep package dry-run on the supported Go floor and do not reintroduce Go 1.24 after the deprecation notice.

## Open risks

- Live GitHub Actions validation has not run in this local slice.
- Windows, macOS, and Linux aarch64 remain `best-effort` until Track 13 provisions runners.
- Historical version-drop diff enforcement remains future validator work; current validator checks the policy structure and proposed-drops table presence.

## Contracts changed

No contract changes were recorded by this Conductor hygiene update.


## Tests added

No tests were added by this Conductor hygiene update.


## Known risks

No new risks were introduced by this Conductor hygiene update.


## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.


## Integration notes

No additional integration notes were recorded by this Conductor hygiene update.
## Phase closeout evidence

Phase closeout evidence refreshed on 2026-05-10:

- Track status advanced from `In Review` to `Done`.
- Review command: `$conductor-review`.
- Review result: no blocking Track 30 findings in the toolchain matrix,
  version-drop policy, workflow trigger coverage, package dry-run Go lane, or
  TypeScript `engines.node` alignment.
- accepted fixes: none required from the 2026-05-10 focused review.
- Focused validator passed:
  - `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1`
- PR integration evidence: PR #18 merged to `origin/main` at
  `98f04cae89fa434a8dcca05d9022db090561c7c7` after all checks passed.
- commit SHA: `b6ceba2fa86ed412325764daef29231b7f6df17d`
- pushed ref: `origin/main`
- `scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: pass after
  closeout commit and push.
- Next-phase decision: Track 30 is `Done`. Reopen only for a scoped support
  matrix change, runner coverage promotion, or version-drop policy update.


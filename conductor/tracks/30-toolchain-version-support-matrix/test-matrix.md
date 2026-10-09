# Test Matrix: Track 30 Toolchain & Version Support Matrix

Last updated: 2026-10-09.

| Check | Alpha | Beta | RC | 1.0 | Current evidence |
|---|---:|---:|---:|---:|---|
| Track docs exist and render cleanly | yes | yes | yes | yes | `spec.md`, `plan.md`, `test-matrix.md`, `risk-register.md`, `handoff.md`, and `validate-toolchain-matrix.ps1` exist. |
| `conductor/toolchain-matrix.md` exists and contains rows for Rust, Python, .NET, Julia, R, Go, Node/Wasm | yes | yes | yes | yes | Static validator checks exact row labels. |
| Each language row includes min version, max version, deprecation horizon, and OS/arch columns | yes | yes | yes | yes | Static validator checks required table headers. |
| Version-drop policy is documented with notice period and removal criteria | yes | yes | yes | yes | Matrix now includes required sequence, removal criteria, exception waiver rule, and proposed drops. |
| `conductor/quality-gates.md` includes `toolchain-matrix-current` and `version-drop-policy-check` | yes | yes | yes | yes | Track 30 gate rows added under Gate definitions. |
| `.github/workflows/toolchain-check.yml` exists and is referenced in CI | yes | yes | yes | yes | Workflow exists and triggers on matrix, gate, workflow, and manifest path changes. |
| `toolchain-check.yml` fails when a CI runner version is outside the declared matrix | yes | yes | yes | yes | Workflow calls `validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem ... -ExpectedPrefix ...`; local Python mismatch probe failed as expected. |
| Go tests and vet run once in binding CI; Go package lane remains covered by declared toolchain floor | yes | yes | yes | yes | `ci-bindings.yml` runs `go test ./...` and `go vet ./...`; `toolchain-check.yml` validates Go `1.25.x` and `1.26.x`. Package dry-run no longer repeats test-only Go work. |
| `toolchain-check.yml` triggers on PRs that modify binding package manifests | yes | yes | yes | yes | Trigger paths include Python, R, Julia, TypeScript, C#, and Go manifest files. |
| Every binding track (06-11) has at least one row in the matrix | yes | yes | yes | yes | Matrix rows map to Tracks 06, 07, 08, 09, 10, and 11. |
| OS/arch cells are labeled as `CI-covered`, `best-effort`, or `unsupported` | yes | yes | yes | yes | Matrix legend and row cells use the accepted labels. |
| Version-drop policy check passes when deprecation notice is present | yes | yes | yes | yes | Proposed drops table records Node 20 and Go 1.24 notice start and earliest removal date. |
| Version-drop policy check fails when a version is removed without notice | partial | yes | yes | yes | Static validator checks policy structure; historical diff comparison remains future work. |
| Matrix is the single source of truth; binding tracks read from it, not define their own floor independently | partial | yes | yes | yes | Matrix documents manifest evidence and says binding tracks must not raise floors or drop versions without policy. |
| Release checklist (Track 15) references the toolchain matrix gate | no | no | yes | yes | Out of current owned scope; handed off to Track 15. |
| Deprecation notice appears in release notes for 2 cycles before removal | no | partial | yes | yes | Matrix policy now requires it; release-note implementation remains Track 15/16 scope. |
| New major language versions are added to the matrix within 1 release cycle | yes | yes | yes | yes | Runner coverage policy now requires refresh within one KairoECS release cycle. |

## C2 current-development qualification profile

The current C2 development profile uses pinned Rust `1.99.0` only. This does
not qualify Cargo-declared older MSRVs, alter a minimum, imply upstream
discontinuation, or relabel historical receipts. No beta runtime lane is part
of this profile.

## Focused Validation Commands

| Command | Result | Evidence |
|---|---|---|
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1` | Expected red pending workflow owner integration | This validator now expects Rust channel `1.99.0` and prefix `1.99`; `.github/workflows/toolchain-check.yml` is owned by Track 13 and still has the prior `stable`/`beta` matrix in this isolated packet. Observed first mismatch: `Missing workflow lane matching matrix: channel: "1.99.0"`; log `.artifacts/c2-current-toolchain-policy/static-validator.log`. Root will validate after integrating that workflow change. |
| `toolchain_bin='/Users/doughnut/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin'; PATH="$toolchain_bin:$PATH"; for tool in rustc cargo rustfmt; do tool_path=$(command -v "$tool") || exit 1; case "$tool_path" in "$toolchain_bin"/*) ;; *) exit 2 ;; esac; version=$("$tool" --version) || exit 1; printf '%s\n' "$tool_path: $version"; done; case "$(rustc --version)" in *1.99.0*) ;; *) exit 3 ;; esac; case "$(cargo --version)" in *1.99.0*) ;; *) exit 4 ;; esac` | Pass | All three executables resolve under the Rustup 1.99.0 ARM64 toolchain; rustc and Cargo report 1.99.0, rustfmt reports its component version. Log `.artifacts/c2-current-toolchain-policy/rust-installed-final.log`. This does not test or qualify older declared MSRVs. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 3.13` | Pass | Local `python --version` reports Python 3.13.x and matches the expected prefix. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem node -ExpectedPrefix 24` | Pass | Local Node reports 24.x. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem dotnet -ExpectedPrefix 11.0` | Pass | Local .NET reports 11.0; the machine currently defaults to preview, not the stable 10.0 SDK lane. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem dotnet -ExpectedPrefix 10.0` | Expected fail | Local .NET reports 11.0; GitHub Actions is expected to install 10.0 and 11.0 lanes explicitly. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem go -ExpectedPrefix 1.26` | Pass | Local Go reports 1.26.x. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 3.11` | Expected fail | Local `python --version` reports Python 3.13.x, proving that prefix mismatch detection is active. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 9.99` | Expected fail | Validator returned `python version mismatch`, proving mismatch detection. |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.

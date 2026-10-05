# Test Matrix: Track 30 Toolchain & Version Support Matrix

Last updated: 2026-10-05.

## Current Rust policy (2026-10-05)

The Kairos owner directed that Rust 1.99.0 is the only supported and pinned Rust version; development follows current stable and does not maintain older Rust compatibility. This Rust-only direction supersedes the prior Rust compatibility notice period. It does not change any other ecosystem policy and does not constitute test, hosted CI, or release acceptance. The dated validation table below is historical and does not establish the new policy gates.

The Track 30 Rust lane requires the exact stable three-part version `1.99.0`; stable/beta aliases, nightly prerelease suffixes, and prefix-only comparisons are rejected. The Track 30 static validator passed. `mise.toml` pins `1.99.0`; the Windows setup validator selects the exact `1.99.0-x86_64-pc-windows-gnu` toolchain, resolves canonical Cargo/rustc/rustdoc executables, checks each exact version, and restores caller environment variables. Its PowerShell regression test extracts production functions and uses mocked tool commands; it passed without launching Rust. See `conductor/evidence/rust199-alias-enforcement-20261005/acceptance.json` and `receipt.json`. Windows-native behavior, actual workspace runtime, hosted CI, and release acceptance remain unverified. Dated validation rows below remain historical and do not establish current acceptance.

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

## Focused Validation Commands

| Command | Result | Evidence |
|---|---|---|
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1` | Pass | Static matrix, workflow trigger, and gate checks passed locally on 2026-05-07. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem rust -ExpectedPrefix 1.99` | Pass | Verified 2026-10-03 on isolated actual Rust 1.99.0 and the PR196 GitHub Actions stable lane at head 5159655; exact local receipts are in the scoped reconciliation below. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 3.13` | Pass | Local `python --version` reports Python 3.13.x and matches the expected prefix. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem node -ExpectedPrefix 24` | Pass | Local Node reports 24.x. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem dotnet -ExpectedPrefix 11.0` | Pass | Local .NET reports 11.0; the machine currently defaults to preview, not the stable 10.0 SDK lane. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem dotnet -ExpectedPrefix 10.0` | Expected fail | Local .NET reports 11.0; GitHub Actions is expected to install 10.0 and 11.0 lanes explicitly. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem go -ExpectedPrefix 1.26` | Pass | Local Go reports 1.26.x. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 3.11` | Expected fail | Local `python --version` reports Python 3.13.x, proving that prefix mismatch detection is active. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 9.99` | Expected fail | Validator returned `python version mismatch`, proving mismatch detection. |
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.

## Historical stable-channel reconciliation — 2026-10-03 (superseded by the 2026-10-05 Rust-only policy)

Official source: `https://static.rust-lang.org/dist/channel-rust-stable.toml`, manifest dated 2026-10-01, Rust 1.99.0 (b940084d7 2026-09-28), SHA-256 `ce6dddc886364f8d786514771212cebe9b731ba82d6b859951c6b0ccc516b6a2`. Hosted PR195 installed 1.99 but expected 1.98. At that time, the current-stable row, workflow prefix and static validator agreed on 1.99; the Rust 1.76 MSRV, beta lane, package manifests and rust-toolchain.toml remained unchanged. The matrix header date remains the last full multi-language refresh; only this Rust row is refreshed here.

On base `34a680cdb738b5e855bac9b821f9adae8c805ed9`, from `/private/tmp/kairos-rust-stable-matrix-20261003`, the static validator passes; actual isolated Rust1.99.0 passes the 1.99 check; actual Rust1.98.1 is rejected with the expected mismatch. actionlint and diff checks pass. Rust1.99 uses isolated `/private/tmp/kairos-rust-matrix-toolchains-20261003`; no shared toolchain defaults changed. Exact commands, environments, exits and logs: `/tmp/rust-matrix-command-receipts.json`. Context was bounded to 24KB; the workflow was inspected separately after the combined packet exceeded budget. A negative attempt using only RUSTUP_TOOLCHAIN did not select the intended compiler on this host; the recorded negative check uses its actual bin directory explicitly.

Narrow CHANGELOG handoff to Track16 records this current-stable observation; no minimum version change or supported-version removal. Hosted checks are still required before merge. No Track30 phase advance or full release qualification is claimed. npm security integration remains with the active parallel owner.

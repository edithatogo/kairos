# Test Matrix: Track 30 Toolchain & Version Support Matrix

Last updated: 2026-09-28.

| Check | Alpha | Beta | RC | 1.0 | Current evidence |
|---|---:|---:|---:|---:|---|
| Track docs exist and render cleanly | yes | yes | yes | yes | `spec.md`, `plan.md`, `test-matrix.md`, `risk-register.md`, `handoff.md`, and `validate-toolchain-matrix.ps1` exist. |
| `conductor/toolchain-matrix.md` exists and contains rows for Rust, Python, .NET, Julia, R, Go, Node/Wasm | yes | yes | yes | yes | Static validator checks exact row labels. |
| Each language row includes min version, max version, deprecation horizon, and OS/arch columns | yes | yes | yes | yes | Static validator checks required table headers. |
| Version-drop policy is documented with notice period and removal criteria | yes | yes | yes | yes | Matrix now includes required sequence, removal criteria, exception waiver rule, and proposed drops. |
| `conductor/quality-gates.md` includes `toolchain-matrix-current` and `version-drop-policy-check` | yes | yes | yes | yes | Track 30 gate rows added under Gate definitions. |
| `.github/workflows/toolchain-check.yml` exists and is referenced in CI | yes | yes | yes | yes | Workflow exists and triggers on matrix, gate, workflow, and manifest path changes. |
| `toolchain-check.yml` fails when a CI runner version is outside the declared matrix | yes | yes | yes | yes | Workflow calls `validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem ... -ExpectedPrefix ...`; local Python mismatch probe failed as expected. |
| `toolchain-check.yml` triggers on PRs that modify binding package manifests | yes | yes | yes | yes | Trigger paths include Python, R, Julia, TypeScript, C#, and Go manifest files. |
| Every binding track (06-11) has at least one row in the matrix | yes | yes | yes | yes | Matrix rows map to Tracks 06, 07, 08, 09, 10, and 11. |
| OS/arch cells are labeled as `CI-covered`, `best-effort`, or `unsupported` | yes | yes | yes | yes | Matrix legend and row cells use the accepted labels. |
| Version-drop policy check passes when deprecation notice is present | yes | yes | yes | yes | Proposed drops table records Node 20 and Go 1.24 notice start and earliest removal date. |
| Version-drop policy check fails when a version is removed without notice | partial | yes | yes | yes | Static validator checks policy structure; historical diff comparison remains future work. |
| Matrix is the single source of truth; binding tracks read from it, not define their own floor independently | partial | yes | yes | yes | Matrix documents manifest evidence and says binding tracks must not raise floors or drop versions without policy. |
| Release checklist (Track 15) references the toolchain matrix gate | no | no | yes | yes | Out of current owned scope; handed off to Track 15. |
| Deprecation notice appears in release notes for 2 cycles before removal | no | partial | yes | yes | Matrix policy now requires it; release-note implementation remains Track 15/16 scope. |
| New major language versions are added to the matrix within 1 release cycle | yes | yes | yes | yes | Runner coverage policy now requires refresh within one KairoECS release cycle. |
| Rust baseline is exact and reproducible | yes | yes | yes | yes | `rust-toolchain.toml` and `mise.toml` pin `1.98.1`; core CI uses the same exact version. |
| Advertised Rust MSRV is exercised | yes | yes | yes | yes | `ci-core.yml` tests the default-feature workspace using `rustup run 1.76.0 cargo test --workspace --locked`. |
| Rust beta canary is date/version qualified and advisory | yes | yes | yes | yes | Beta lane expects `1.99.0-beta.8` observed 2026-09-27 and uses `continue-on-error`; exact version mismatch is reported. |
| Rust exact-version mismatch is rejected | yes | yes | yes | yes | `validate-toolchain-matrix.ps1 -ExpectedVersion` compares the complete Rust version string. |

## Focused Validation Commands

| Command | Result | Evidence |
|---|---|---|
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1` | Pass | Static matrix, workflow trigger, and gate checks passed locally on 2026-05-07. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem rust -ExpectedVersion 1.98.1` | Pass | Exact tested baseline check; see dated validation receipt below. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem rust -ExpectedVersion 1.99.0-beta.8` | Pass when beta matches snapshot; otherwise advisory mismatch | Dated beta canary snapshot observed 2026-09-27; exact mismatch is non-blocking in CI. |
| `rustup run 1.76.0 cargo test --workspace --locked` | Pass: 228 tests, 64 suites | `conductor/evidence/d1.2-msrv-rust-1.76.0-20260928.log`; SHA-256 `97ac7324f8b9eab2a8eb169f905e7851af148b5ae2f3d0e8b6eab7db5a165a2f` |
| `rustup run 1.98.1 cargo test --workspace --locked` | Pass: 228 tests, 64 suites | `conductor/evidence/d1.2-stable-rust-1.98.1-20260928.log`; SHA-256 `fcce663653865ffcd1cd6c38d5d578245dd96e2176e55f1ac92bffecddaf1d2d` |
| `rustup run beta cargo check --workspace --locked` | Pass: Rust 1.99.0-beta.8 | `conductor/evidence/d1.2-beta-rust-1.99.0-beta.8-20260928.log`; SHA-256 `a208e13ce33e23c5b576131115c6a02b5348792ba53cbae0924bff55b580e09b` |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 3.13` | Pass | Local `python --version` reports Python 3.13.x and matches the expected prefix. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem node -ExpectedPrefix 24` | Pass | Local Node reports 24.x. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem dotnet -ExpectedPrefix 11.0` | Pass | Local .NET reports 11.0; the machine currently defaults to preview, not the stable 10.0 SDK lane. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem dotnet -ExpectedPrefix 10.0` | Expected fail | Local .NET reports 11.0; GitHub Actions is expected to install 10.0 and 11.0 lanes explicitly. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem go -ExpectedPrefix 1.26` | Pass | Local Go reports 1.26.x. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 3.11` | Expected fail | Local `python --version` reports Python 3.13.x, proving that prefix mismatch detection is active. |
| `pwsh -NoProfile -File conductor/tracks/30-toolchain-version-support-matrix/validate-toolchain-matrix.ps1 -CheckInstalled -Ecosystem python -ExpectedPrefix 9.99` | Expected fail | Validator returned `python version mismatch`, proving mismatch detection. |
## D1.2 toolchain and security receipts — 2026-09-28

- Rust 1.76.0 workspace: 228 tests, 64 suites passed; log SHA-256
  `97ac7324f8b9eab2a8eb169f905e7851af148b5ae2f3d0e8b6eab7db5a165a2f`.
- Rust 1.98.1 workspace: 228 tests, 64 suites passed; log SHA-256
  `fcce663653865ffcd1cd6c38d5d578245dd96e2176e55f1ac92bffecddaf1d2d`.
- Rust 1.99.0-beta.8 `cargo check --workspace --locked`: passed; log SHA-256
  `a208e13ce33e23c5b576131115c6a02b5348792ba53cbae0924bff55b580e09b`.
- Track 13 supply-chain script: metadata, cargo-deny and cargo-audit 0.22.1
  passed; raw log SHA-256
  `b8ab651da5cd1ed7593952d462dce03098faae8950f57f7c84f23b6e51be74e0`.
- RUSTSEC-2026-0204: patched lock pin `crossbeam-epoch 0.9.20` (MSRV 1.61);
  no package manifest change.
- `actionlint` on the eight affected workflows and `git diff --check`: passed.
  Hosted Actions and Windows remain unverified.

## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.

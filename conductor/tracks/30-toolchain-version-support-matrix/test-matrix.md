# Test Matrix: Track 30 Toolchain & Version Support Matrix

| Check | Alpha | Beta | RC | 1.0 |
|---:|---:|---:|---:|---:|
| Track docs exist and render cleanly | yes | yes | yes | yes |
| `conductor/toolchain-matrix.md` exists and contains rows for Rust, Python, .NET, Julia, R, Go, Node/Wasm | yes | yes | yes | yes |
| Each language row includes min version, max version, deprecation horizon, and OS/arch columns | yes | yes | yes | yes |
| Version-drop policy is documented with notice period and removal criteria | yes | yes | yes | yes |
| `conductor/quality-gates.md` includes `toolchain-matrix-current` and `version-drop-policy-check` | yes | yes | yes | yes |
| `.github/workflows/toolchain-check.yml` exists and is referenced in CI | yes | yes | yes | yes |
| `toolchain-check.yml` fails when a CI runner version is outside the declared matrix | yes | yes | yes | yes |
| `toolchain-check.yml` triggers on PRs that modify binding package manifests | no | yes | yes | yes |
| Every binding track (06-11) has at least one row in the matrix | no | yes | yes | yes |
| OS/arch cells are labeled as CI-covered, best-effort, or unsupported | no | yes | yes | yes |
| Version-drop policy check passes when deprecation notice is present | no | yes | yes | yes |
| Version-drop policy check fails when a version is removed without notice | no | yes | yes | yes |
| Matrix is the single source of truth — binding tracks read from it, not define their own floor independently | no | yes | yes | yes |
| Release checklist (Track 15) references the toolchain matrix gate | no | no | yes | yes |
| Deprecation notice appears in release notes for 2 cycles before removal | no | no | yes | yes |
| New major language versions are added to the matrix within 1 release cycle | no | no | yes | yes |

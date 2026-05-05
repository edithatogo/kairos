# Threat Model

## Assets

- Native libraries distributed through GitHub Releases and package registries
- Python wheels, npm packages, NuGet packages, Rust crates, R/Julia/Go packages
- User simulation data and telemetry outputs
- CI/CD credentials and registry publishing permissions
- Reputation of reproducibility/performance claims

## Threats

| Threat | Mitigation |
|---|---|
| Malicious dependency update | Dependabot/Renovate review, lockfiles, cargo-deny, OSV scanning |
| Compromised GitHub Action | Pin actions, actionlint, zizmor, minimal permissions |
| Registry token leakage | Prefer OIDC/Trusted Publishing where available; scoped tokens otherwise |
| Native library substitution | checksums, signatures, provenance, CI-built artifacts only |
| FFI memory corruption | fuzzing, sanitizers, handle lifecycle tests, SafeHandle/finalizers |
| Benchmark manipulation | public harness, raw outputs, version-pinned competitors |
| Misleading docs | maturity labels and release gate review |
| Maintainer account compromise | 2FA requirement, protected environments, CODEOWNERS |

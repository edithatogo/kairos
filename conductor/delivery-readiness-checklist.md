# KairoECS Delivery Readiness Checklist

Use this before every alpha, beta, release candidate, and stable release.

## Product readiness

- [ ] Core DES fixture works.
- [ ] Core ABM fixture works.
- [ ] Hybrid DES/ABM fixture works.
- [ ] 1M entity/event benchmark recorded.
- [ ] Arrow telemetry works.
- [ ] FFI lifecycle works.

## Binding readiness

- [ ] Python binding package surface exists and Python 3.10 green.
- [ ] Python 3.11 green.
- [ ] Python 3.12 green.
- [ ] Python 3.13 green.
- [ ] Python 3.14 green.
- [ ] R binding package surface exists and package check is green.
- [ ] Julia binding package surface exists and package tests are green.
- [ ] TypeScript/Wasm binding package surface exists and tests are green.
- [ ] C# binding package surface exists and net10.0 green.
- [ ] C# net11.0 green or explicitly marked experimental.
- [ ] Go binding package surface exists and tests are green.

## Documentation readiness

- [ ] GitHub Pages site builds.
- [ ] Install page complete.
- [ ] Quickstarts for all languages.
- [ ] Factory bottleneck tutorial.
- [ ] Flocking behavior tutorial.
- [ ] Hybrid tutorial.
- [ ] FFI docs.
- [ ] Arrow schema docs.
- [ ] Release/compatibility docs.

## Governance readiness

- [ ] LICENSE.
- [ ] SECURITY.md.
- [ ] CONTRIBUTING.md.
- [ ] CODE_OF_CONDUCT.md.
- [ ] CODEOWNERS.
- [ ] Maintainers documented.
- [ ] Naming due diligence complete.
- [ ] Compatibility policy names the live Rust and binding package roots.
- [ ] `docs/design/protected-surface-inventory.json` names every protected Rust, C ABI, Arrow, host API, and conformance root.
- [ ] `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1` passes for the policy pack.
- [ ] `pwsh -NoProfile -File docs/design/validate-compatibility-pack.ps1 -ReleaseGate` passes before beta, RC, or 1.0.
- [ ] Breaking protected-surface changes have ADRs before merge.
- [ ] Breaking protected-surface changes have migration notes before beta, RC, or 1.0.
- [ ] Release notes and compatibility notes name the exact affected protected roots.
- [ ] OpenSSF/supply-chain readiness rows from Track 20 are green.
- [ ] API compatibility review rows from Track 25 are green.
- [ ] Red-team blocker rows from Track 28 are green or explicitly waived.
- [ ] Wave manager controls from Track 29 are green.
- [ ] Toolchain matrix and version-drop policy from Track 30 are green.
- [ ] Performance regression guard from Track 31 is green or explicitly marked advisory.

### OpenSSF and supply-chain readiness

- [ ] `SECURITY.md`, `CODEOWNERS`, and `.github/CODEOWNERS` exist.
- [ ] Dependency automation exists through `.github/dependabot.yml` or `renovate.json`.
- [ ] `.github/workflows/scorecard.yml` exists and runs OpenSSF Scorecard on `main`.
- [ ] `.github/workflows/dependency-review.yml` exists and includes `fail-on-severity: high`.
- [ ] `.github/workflows/actions-security.yml` and `.github/workflows/workflow-security.yml` exist for workflow hardening.
- [ ] `.github/workflows/secret-scan.yml` exists and fails on findings.
- [ ] `.github/workflows/sbom-attestations.yml` exists and can emit `sbom.spdx.json`.
- [ ] `.github/workflows/sbom-attestations.yml` verifies `RELEASE.txt`, `release-artifact-manifest.json`, and `SHA256SUMS` before attesting SBOM evidence.
- [ ] `.github/workflows/release-attestations.yml` exists and can attest the release artifact tree.
- [ ] Release artifact tree includes `RELEASE.txt`, `SHA256SUMS`, and `sbom.spdx.json` before RC or 1.0.
- [ ] `SECURITY.md` documents vulnerability acknowledgement, private disclosure, and exception recording expectations.
- [ ] Release notes name SBOM, checksum, provenance/attestation evidence, and any approved exception.
- [ ] Exceptions follow `conductor/tracks/20-openssf-supply-chain-institutional-trust/supply-chain-plan.md`.
- [ ] Temporary allowed-failure lanes are stage-limited and do not carry into RC or 1.0 without explicit approval.

## Publishing readiness

- [ ] crates.io dry-run.
- [ ] TestPyPI/PyPI dry-run.
- [ ] npm dry-run.
- [ ] NuGet pack validation.
- [ ] R-universe or R package check.
- [ ] Julia artifact/registry dry-run.
- [ ] Go tag smoke.
- [ ] GitHub Release artifacts.
- [ ] Checksums.
- [ ] SBOM/provenance.

## Maintenance readiness

- [ ] Dependency automation active.
- [ ] Security scanning active.
- [ ] Release cadence documented.
- [ ] Deprecation policy documented.
- [ ] Compatibility policy documented.
- [ ] Issue triage labels created.

## Community and research readiness

- [ ] CITATION.cff validates.
- [ ] codemeta.json exists.
- [ ] Zenodo metadata reviewed.
- [ ] JOSS paper skeleton updated if research release.
- [ ] Comparative benchmarks and reproducibility guidance from Track 18 is green.
- [ ] Research software, citation, and archival guidance from Track 19 is green.
- [ ] OpenSSF, supply-chain trust, and institutional-readiness guidance from Track 20 is green.
- [ ] Community adoption, education, and ecosystem guidance from Track 17 is green.
- [ ] Verification, validation, and uncertainty guidance from Track 21 is green.
- [ ] Scenario runner and replay guidance from Track 22 is green.
- [ ] Model zoo includes maturity labels.
- [ ] Starter-kit and model-zoo guidance from Track 23 is green.
- [ ] At least one DES, one ABM, and one hybrid example run end-to-end.
- [ ] Playground and demo guidance from Track 24 is green.
- [ ] API design review and compatibility governance guidance from Track 25 is green.
- [ ] Interoperability standards review guidance from Track 26 is green.
- [ ] Streaming and real-time guidance from Track 36 is green.
- [ ] ML/AI integration guidance from Track 37 is green.
- [ ] FMI/FMU digital twin guidance from Track 38 is green.
- [ ] Cloud/HPC batch runner guidance from Track 39 is green.
- [ ] Time-travel debugging guidance from Track 40 is green.
- [ ] Benchmark scripts are public before benchmark claims appear in docs.
- [ ] Reproducibility command is documented for every flagship example.

## Red-team readiness

- [ ] `reviews/red-team-report.md` reviewed.
- [ ] `reviews/red-team-report.md` includes a freshness date within 14 days of release planning or was re-run for the target beta/RC/1.0 gate.
- [ ] Claim-versus-capability ledger rows name evidence, verdict, owner, stage impact, and counterexample language to avoid.
- [ ] Blocker rubric is applied: Blocker, Warning, or Note.
- [ ] All critical red-team findings resolved or explicitly accepted.
- [ ] Every blocker or warning has a named owner and follow-up path.
- [ ] Missing release evidence paths are downgraded in release notes or explicitly accepted by the release manager.
- [ ] Devil's advocate objections are addressed in release notes.
- [ ] Any preview/experimental claims are labelled clearly.
- [ ] .NET 11 lane is marked preview/experimental until GA.
- [ ] Arrow zero-copy claims specify exact copy/lifetime behavior.
- [ ] FFI memory safety tests passed for published bindings.
- [ ] `dist/release-artifact-manifest.json`, `dist/SHA256SUMS`, and SBOM/provenance evidence exist before RC or 1.0 native artifact claims.
- [ ] Red-team validation commands from `conductor/tracks/28-red-team-devils-advocate-review/test-matrix.md` have been run for this release stage.
- [ ] Wave 5 control tracks are current.

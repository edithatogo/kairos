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
- [ ] OpenSSF/supply-chain readiness rows from Track 20 are green.
- [ ] API compatibility review rows from Track 25 are green.
- [ ] Red-team blocker rows from Track 28 are green or explicitly waived.

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
- [ ] Benchmark scripts are public before benchmark claims appear in docs.
- [ ] Reproducibility command is documented for every flagship example.

## Red-team readiness

- [ ] `reviews/red-team-report.md` reviewed.
- [ ] All critical red-team findings resolved or explicitly accepted.
- [ ] Devil's advocate objections are addressed in release notes.
- [ ] Any preview/experimental claims are labelled clearly.
- [ ] .NET 11 lane is marked preview/experimental until GA.
- [ ] Arrow zero-copy claims specify exact copy/lifetime behavior.
- [ ] FFI memory safety tests passed for published bindings.
- [ ] Red-team freshness date is current for the release stage being planned.
- [ ] Release blockers have named owners and follow-up issues.

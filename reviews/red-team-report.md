# KairoECS Red-Team Report

## Executive summary

KairoECS is credible as a repository, but release claims still outrun the current evidence in several places. The checked-in Rust workspace, binding package surfaces, CI workflows, docs, and readiness gates are real; what remains risky is treating those surfaces as finished product claims before their gate rows are green and their smoke checks are reproducible.

The release posture should stay staged: kernel and conformance first, then binding/package dry-runs, then docs and release-evidence claims, then public publication.

## Claim-vs-capability ledger

| Claim | Current repository capability | Release risk | Planning consequence |
|---|---|---|---|
| Rust core and state are ready for public release | `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, and `crates/kairo-ecs-rng` are checked in, and `ci-core.yml`, `package-dry-run.yml`, `benchmark-smoke.yml`, `fuzzing.yml`, and `conformance.yml` exist | The code exists and the repo now has concrete conformance and packaging lanes, but release claims still depend on those checks passing for the target surface | Keep Rust core release-gated until conformance, package dry-runs, and benchmark smoke checks are verified in CI |
| Python/R/Julia/TypeScript/C#/Go are production-ready | The six binding package roots exist and are wired into `ci-bindings.yml` and `package-dry-run.yml` | CI currently proves smoke checks, manifest presence, and dry-run packaging, not ecosystem maturity or long-term compatibility | Treat all non-Rust package roots as release-gated surfaces, not stable promises, until their per-binding checks pass |
| Benchmarks are reproducible and publication-ready | `benches/benchmark-plan.md`, `conformance/fixtures/manifest.json`, and `.github/workflows/benchmark-smoke.yml` exist | The smoke workflow proves the harness is present and invokable; it does not itself prove comparison quality or fairness | Do not publish benchmark claims without the benchmark plan, fixture IDs, and raw outputs staying versioned together |
| Conformance fixtures are current enough to support binding and scheduler claims | `conformance/fixtures/manifest.json` and `.github/workflows/conformance.yml` validate the ready fixture set and canonical benchmark names | The workflow proves only the checked-in ready fixtures and named scenarios; planned fixtures remain future work | Keep conformance claims tied to the ready fixture IDs and canonical benchmark scenarios only |
| Fuzzing coverage is complete enough to support safety claims | `.github/workflows/fuzzing.yml` exists and invokes `cargo fuzz run ffi_boundary` if `fuzz/` is present | The workflow is a smoke gate, not proof of exhaustive coverage | Keep safety claims scoped to the checked-in fuzz target and its current lane only |
| Docs maturity and public guidance are done | `website/src/index.md`, `docs/trustworthy-simulation/replay-and-seeds.md`, `docs/trustworthy-simulation/verification-validation-uncertainty.md`, `conductor/community-adoption.md`, `conductor/experiment-runner.md`, and `conductor/domain-model-zoo.md` exist | The docs are present, but they are guidance documents and landing/index surfaces, not proof that every referenced capability is implemented end-to-end | Keep docs language aligned to current capability and avoid implying a finished release unless the readiness rows are green |
| OpenSSF and supply-chain readiness are done | `conductor/delivery-readiness-checklist.md`, `conductor/quality-gates.md`, and the concrete `sbom-attestations.yml` / `release-attestations.yml` workflows explicitly call out SBOM, provenance, dependency review, and waiver handling | The repository now has actual attestation workflows, but that is still not the same as shipped release evidence | Treat the gate rows and attestation workflows as blockers until the corresponding artifacts are present and passing for the target release |
| API compatibility governance is complete | `conductor/contracts/versioning-compatibility.md`, `conductor/delivery-readiness-checklist.md`, and `conductor/quality-gates.md` name compatibility policy, ADRs, and migration notes | The policy exists as text; the repo still needs releases to prove it is being enforced | Do not allow package-root renames or breaking API claims without a recorded ADR and migration note |
| Interoperability standards review is complete | Track 26 exists and names a standards inventory plus mapping table | The track is review-oriented; it does not itself guarantee broad cross-ecosystem parity | Avoid claiming full interoperability beyond the mapped, supported translations |

## Top release-blocking risks and patch status

| Risk | Severity | Red-team critique | Countermeasure included in this pack | Status |
|---|---:|---|---|---|
| Scope explosion | Critical | The repo spans a kernel, six bindings, docs, release policy, and research credibility surfaces | Stage release claims and keep public promises behind readiness gates | Patched |
| Binding maturity mismatch | High | Smoke-tested package roots are not the same as production-ready language products | Keep non-Rust packages release-gated until their package and CI checks are green | Patched |
| Benchmark overclaim | High | A benchmark smoke job can exist without a defensible comparison harness | Publish benchmark claims only with the benchmark plan, fixture IDs, and outputs | Patched |
| Fuzzing overclaim | High | A single fuzz target does not justify broad safety claims | Scope safety claims to the current fuzz lane and its limits | Patched |
| Documentation overclaim | High | Docs can look authoritative while the underlying capability is still partial | Keep docs maturity labels synchronized with the readiness checklist | Patched |
| Supply-chain overclaim | High | A checklist row is not the same as provenance, SBOM, or scorecard evidence | Require concrete attestation workflows and artifact checks before release claims | Patched |
| Compatibility overclaim | High | Compatibility policy text can drift from real package-root behavior | Require ADRs and migration notes before breaking changes or renames | Patched |
| Interoperability overclaim | Medium | Mapping review can be mistaken for broad ecosystem support | Keep Track 26 limited to mapped translations and known gaps | Patched |

## Adversarial release scenario

A realistic failure mode:

1. The project publishes Rust and Python artifacts because the package roots exist.
2. The docs and release notes describe the bindings as ready for general use.
3. The benchmark page shows a comparison, but the raw fixture set is not pinned alongside it.
4. The R/Julia/TypeScript/C#/Go package surfaces are treated as stable rather than release-gated.
5. A later API rename slips past without an ADR or migration note.
6. Community trust drops because the repository promised more than the current gates prove.

Mitigation in this pack:

```text
- keep public claims behind the readiness checklist
- keep benchmark claims tied to fixture IDs and benchmark-plan.md
- keep conformance claims tied to the ready fixture manifest and canonical scenarios
- keep binding package roots release-gated until CI and package dry-runs pass
- keep supply-chain claims tied to the attestation workflows and release artifacts
- keep compatibility changes behind ADRs and migration notes
- keep interoperability claims limited to the standards map
```

## Recommended hard pivots

1. Treat the checked-in Rust workspace as the first releaseable technical core, not the entire product story.
2. Keep all six binding package roots as gated surfaces until their smoke and package checks are green in CI.
3. Publish benchmark claims only after the benchmark harness, fixture manifest, and comparison story stay versioned together.
4. Keep supply-chain work anchored to the attestation workflows and release artifacts, not just checklist language.
5. Keep compatibility work as an explicit release blocker until ADR and migration-note requirements are satisfied.
6. Keep interoperability review scoped to documented mappings and gaps.
7. Do not let docs overstate maturity; the readiness checklist is the release truth source.

## Red-team release gate

No release candidate may ship unless:

- all critical risks have explicit acceptance or mitigation
- conformance fixtures pass for every package included in that release
- package dry-runs pass for every published root included in that release
- SBOM and release attestation workflows complete for the release artifacts
- docs maturity labels match implemented behavior
- public performance claims link to reproducible benchmarks
- package names and legal/trademark checks are complete
- checksums/provenance artifacts are attached for native releases
- red-team issue list has no unresolved Critical or unaccepted High risk

# KairoECS Red-Team Report

## Executive summary

KairoECS is credible as a repository, but release claims still outrun the current evidence in several places. The checked-in Rust workspace, binding package surfaces, CI workflows, docs, and readiness gates are real; what remains risky is treating those surfaces as finished product claims before their gate rows are green and their smoke checks are reproducible.

The release posture should stay staged: kernel and conformance first, then binding/package dry-runs, then docs and release-evidence claims, then public publication.

## Review metadata

| Field | Value |
|---|---|
| Freshness date | 2026-05-06 |
| Freshness window | Re-run for every beta, RC, or 1.0 release plan; otherwise stale after 14 days |
| Machine-readable ledger | `conductor/tracks/28-red-team-devils-advocate-review/claim-capability-ledger.json` |
| Evidence commands | `Test-Path` and `Get-Content` commands recorded in `conductor/tracks/28-red-team-devils-advocate-review/test-matrix.md` |
| Required owner check | Every blocker or warning row must name a track owner, subagent, or release role |

## Claim-versus-capability ledger

| ID | Public or planning claim | Capability evidence checked | Current verdict | Owner | Stage impact | Counterexample that must stay out of release language |
|---|---|---|---|---|---|---|
| CVC-01 | Rust core and state are ready for public release | `Cargo.toml`; `crates/kairo-ecs-types/`; `crates/kairo-ecs-core/`; `crates/kairo-ecs-state/`; `crates/kairo-ecs-rng/`; `.github/workflows/ci-core.yml`; `.github/workflows/conformance.yml`; `.github/workflows/package-dry-run.yml` | Supported only as a gated Rust-core candidate | core-scheduler-agent, ecs-agent, release-agent | Blocks beta+ if conformance or package dry-run is absent/failing | "Rust core is production-ready" without current CI and conformance evidence |
| CVC-02 | Python/R/Julia/TypeScript/C#/Go are production-ready | `bindings/python/`; `bindings/r/`; `bindings/julia/`; `bindings/typescript/`; `bindings/csharp/`; `bindings/go/`; `.github/workflows/ci-bindings.yml`; `packaging/release-package-manifest.json` | Not supported as production-ready; supported only as staged or preview surfaces | binding owners, release-agent | Blocks RC/1.0 for any published binding with failed smoke/package checks | "Six stable bindings" because package roots exist |
| CVC-03 | Benchmarks are reproducible and publication-ready | `benches/benchmark-plan.md`; `conformance/fixtures/manifest.json`; `.github/workflows/benchmark-smoke.yml` | Supported only as a smoke/reproducibility plan until raw outputs are versioned | performance-agent | Blocks public benchmark claims at alpha+ | "Fastest" or comparative graph without raw command, fixture IDs, and environment |
| CVC-04 | Conformance fixtures support scheduler and binding claims | `conformance/fixtures/manifest.json`; `.github/workflows/conformance.yml` | Supported for ready fixtures only: `scheduler_ordering_v1`, `scheduler_cancellation_v1`, `rng_reproducibility_v1`, `vvuq_scenario_replay_v1` | conformance-agent | Blocks beta+ claims outside ready fixture coverage | Claiming DES, ABM, hybrid, Arrow, or FFI fixture conformance while those fixture IDs remain planned |
| CVC-05 | Fuzzing coverage supports safety claims | `.github/workflows/fuzzing.yml`; optional `fuzz/` target presence | Not supported as broad safety coverage; supported as a smoke lane only | security-agent, conformance-agent | Blocks RC/1.0 native/FFI safety claims unless scoped | "Memory safe across all FFI bindings" from one smoke lane |
| CVC-06 | Docs maturity and public guidance are complete | `website/src/index.md`; `docs/trustworthy-simulation/`; `docs/community/`; `conductor/delivery-readiness-checklist.md` | Supported as guidance surfaces, not implementation proof | docs-agent | Blocks beta+ if docs imply features whose gate rows are not green | Quickstarts or docs claiming future binding/package availability as current |
| CVC-07 | OpenSSF and supply-chain readiness are done | `SECURITY.md`; `CODEOWNERS`; `.github/workflows/scorecard.yml`; `.github/workflows/dependency-review.yml`; `.github/workflows/sbom-attestations.yml`; `.github/workflows/release-attestations.yml` | Supported as gate infrastructure; not supported as shipped release evidence | security-agent, release-agent | Blocks RC/1.0 native artifacts without SBOM/checksum/provenance evidence | "SLSA/OpenSSF ready" before generated artifacts are attached |
| CVC-08 | API compatibility governance is complete | `conductor/contracts/versioning-compatibility.md`; `docs/release/compatibility.md`; `docs/adr/`; `conductor/delivery-readiness-checklist.md` | Supported as policy; enforcement must be proven per release | api-governance-agent, release-agent | Blocks RC/1.0 for renamed or breaking public roots without ADR/migration note | Breaking package root rename presented as a minor release detail |
| CVC-09 | Interoperability standards review is complete | `docs/interoperability/standards-review.md`; Track 26 references | Supported only as a standards/gap map | interoperability-agent | Blocks interoperability claims beyond documented mappings | Claiming semantic compatibility with Mesa, Agents.jl, MASON, NetLogo, or DEVS tooling |
| CVC-10 | Release artifact evidence exists for the current release train | `docs/release/release-checklist.md`; `packaging/release-package-manifest.json`; expected `dist/release-artifact-manifest.json` | Not yet supported: `dist/release-artifact-manifest.json` was absent in the focused check | release-agent | Blocks RC/1.0 publication | Release notes saying artifacts, checksums, and SBOM are attached before the manifest exists |

## Blocker rubric

| Class | Definition | Required action | Release effect |
|---|---|---|---|
| Blocker | Unsupported or false claim affects a public release surface, safety/security statement, package publication, compatibility promise, or benchmark/comparison claim | Remove/downgrade the claim, produce evidence, or record explicit release-manager acceptance with owner and expiry | Blocks beta, RC, and 1.0 unless accepted; Critical blockers also block alpha |
| Warning | Claim is directionally true but narrower than stated, stale, missing owner, or backed only by smoke/checklist evidence | Rewrite the claim with maturity labels and add owner/follow-up | Does not block alpha; blocks RC/1.0 if still unresolved |
| Note | Concern is real but not tied to current release language or release artifacts | Track as handoff/risk only | Does not block release unless it becomes release-facing |

## Freshness and owner validation

The report is current only when all of the following are true:

- the freshness date is within 14 days of release planning, or the report was re-run for the specific beta/RC/1.0 gate
- every `Blocker` or `Warning` row in the claim ledger names an owner
- every release-facing blocker has a matching row in `conductor/delivery-readiness-checklist.md`
- any missing evidence path is either downgraded in release language or explicitly accepted by the release manager
- the local validation commands in the Track 28 test matrix have been run and recorded

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

# KairoECS Red-Team Review

## Release posture

The repository has real package roots, workflows, docs, and readiness gates, but the release narrative must stay narrower than the current surface area. The release-blocking question is not whether the files exist; it is whether the checked-in capabilities match the claims made in release planning.
The claim-vs-capability boundary now includes concrete conformance validation, package dry-runs, and release attestation workflows, not just placeholder gate language.

## Current release blockers

- Rust core, bindings, docs, benchmarks, and supply-chain gates are all present, but claims must stay behind the readiness checklist until the corresponding rows are green.
- The six binding package roots are smoke-tested surfaces, not blanket production promises.
- Benchmark and reproducibility claims must stay tied to `benches/benchmark-plan.md`, `conformance/fixtures/manifest.json`, and the benchmark smoke workflow.
- Conformance claims must stay tied to the ready fixture manifest and the conformance workflow that validates those fixture IDs and canonical benchmark names.
- Supply-chain claims must stay tied to `sbom-attestations.yml`, `release-attestations.yml`, and the release artifact manifest/checksum path, not just to checklist prose.
- Compatibility and interoperability claims should be treated as release blockers until the relevant checklist rows and track docs are current.
- Release artifact evidence is not present until `dist/release-artifact-manifest.json`, `dist/SHA256SUMS`, and the SBOM/provenance outputs exist for the target release train.

## Blocker rubric

| Class | Trigger | Owner requirement | Release effect |
|---|---|---|---|
| Blocker | Unsupported claim touches package publication, safety/security, compatibility, benchmark comparison, native artifacts, or release readiness | Must name a remediation owner and release-manager decision owner | Blocks beta, RC, and 1.0 unless evidence is produced or explicitly accepted |
| Warning | Claim is partly true but overbroad, stale, missing maturity label, or backed only by smoke/checklist evidence | Must name a track owner or subagent | Blocks RC/1.0 if unresolved |
| Note | Concern is not release-facing for the planned stage | Owner optional but preferred | Handoff only |

## Freshness rule

`reviews/red-team-report.md` is stale if its freshness date is older than 14 days or if beta, RC, or 1.0 planning has started since the last review. A stale report is treated as missing release evidence until Track 28 re-runs the ledger, confirms owners, and records the validation commands in the track test matrix.

## Planning rules

1. Do not claim general release readiness because a package root or workflow exists.
2. Do not claim benchmark credibility without pinned fixtures and a reproducible comparison path.
3. Do not claim conformance coverage beyond the ready fixtures and canonical benchmark names that the workflow validates.
4. Do not claim compatibility stability without ADRs, migration notes, and explicit package-root alignment.
5. Do not claim interoperability beyond the standards map and documented gaps.
6. Use the readiness checklist as the source of truth for release planning.
7. Do not accept a blocker row without an owner, evidence path, and stage impact.

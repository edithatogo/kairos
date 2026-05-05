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

## Planning rules

1. Do not claim general release readiness because a package root or workflow exists.
2. Do not claim benchmark credibility without pinned fixtures and a reproducible comparison path.
3. Do not claim conformance coverage beyond the ready fixtures and canonical benchmark names that the workflow validates.
4. Do not claim compatibility stability without ADRs, migration notes, and explicit package-root alignment.
5. Do not claim interoperability beyond the standards map and documented gaps.
6. Use the readiness checklist as the source of truth for release planning.

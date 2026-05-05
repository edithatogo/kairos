# KairoECS Community + Red-Team SOTA Pack Manifest

This pack extends the prior KairoECS Conductor SOTA setup with community adoption, reproducibility, research software, supply-chain trust, experiment management, model zoo, interoperability, developer experience, and red-team review.

## Key additions

- Tracks 17-28 with `spec.md`, `plan.md`, `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.
- `reviews/red-team-report.md`, `reviews/devils-advocate-review.md`, `reviews/threat-model.md`, and release red-team checklist.
- `conductor/package-ecosystem-plan.md` with detailed tool/package planning for Rust, Python 3.10-3.14, R, Julia, TypeScript/Wasm, C# .NET 10-11, Go, docs, and security.
- `conductor/trustworthy-simulation.md` and `conductor/continuous-assessment.md`.
- Model zoo scaffolds under `examples/`.
- Research software metadata: `CITATION.cff`, `codemeta.json`, `.zenodo.json`, `paper/`.
- Governance files and contributor onboarding.
- Additional GitHub Actions for Scorecard, docs quality, package dry runs, conformance placeholders, benchmark smoke, workflow security, and attestation/SBOM planning.
- Developer environment scaffolds: `.devcontainer/`, `devbox.json`, `mise.toml`, `justfile`, `scripts/bootstrap.sh`.
- Additional Mermaid diagrams under `planning/diagrams/`.

## Red-team verdict

The recommended v0.1 release should be narrower than the full roadmap: Rust core, stable C ABI preview, Python preview, Arrow event log, conformance fixtures, docs site, and three examples. Other bindings should remain scaffolded until the FFI/Arrow contracts stabilize.


## KairoECS naming update

This revision renames the package ecosystem from the earlier Kairos/KairosSim planning language to **KairoECS**. The preferred public package family is now:

```text
repo: kairo-ecs
Rust root crate: kairo-ecs
Rust internal crates: kairo-ecs-core, kairo-ecs-state, kairo-ecs-ffi, kairo-ecs-arrow, kairo-ecs-viz
Python distribution: kairo-ecs
Python import: kairo_ecs
R package: kairoECS
Julia package: KairoECS.jl
npm scope: @kairo-ecs/core
NuGet: Kairo.ECS
Go module: github.com/<org>/kairo-ecs
C ABI: libkairo_ecs / kairo_ecs.h / kairo_ecs_*
CLI: kairoecs
```

File count in this pack: **362**.

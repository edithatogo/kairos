# Continuous Assessment Log

Subagents should append to this file rather than losing review insights in chat.

## Current staged path

```text
v0.1: deterministic Rust scheduler + ECS + seed/replay skeleton
v0.2: Python preview + Arrow event logs + DES/ABM examples
v0.3: TypeScript/Wasm preview + playground + benchmark publication
v0.4: R/Julia/C#/Go previews + conformance suite expansion
v0.5: experiment runner + model zoo + release hardening
v1.0: stable C ABI, stable Arrow schemas, public compatibility promise
```

## Incorporated recommendations

- Community/adoption tracks.
- Research citation and archival metadata.
- OpenSSF/Scorecard/SBOM/attestations.
- V&V, uncertainty, and deterministic replay tracks.
- Experiment runner and scenario management.
- Domain starter kits and model zoo.
- Playground and browser demos.
- Cross-language API review gates.
- Standards/interoperability review.
- Red-team and devil's advocate reviews.

## Still worth considering later

| Area | Recommendation | Stage |
|---|---|---|
| Funding | OpenCollective/GitHub Sponsors after naming due diligence | after v0.2 |
| Governance | Scientific steering group or advisory board | after community traction |
| Formal methods | Kani/loom checks for scheduler invariants | after core stabilizes |
| PDES | Conservative parallel DES research spike | after sequential determinism is proven |
| Digital twins | FMI/FMU import/export spike | after experiment runner |
| RL | Gymnasium-compatible Python environment | after model zoo |
| Cloud | Batch/HPC runners and container images | after release maturity |
| Education | University course module/tutorial set | after docs site |

## 2026-05-05: Conductor system review

1. Toolchain targets: Python 3.14 preview, .NET 11 preview, Rust stable — all current as of May 2026.
2. Registry policies: No changes detected. Naming due diligence still pending for all target registries.
3. UniFFI/Diplomat: Still the best generated binding tools. No alternatives identified.
4. C ABI: Minimal and stable by design. 15-function surface defined in ffi-contract.md.
5. Benchmarks: Fairness criteria defined in benchmark-policy.md. No comparative results yet.
6. Docs: Ahead of implementation. Specs exist for all tracks. Core implementation catching up.
7. Visualization: Not over-invested. Track 05 is Planned and non-blocking.
8. Setup complexity: devbox.json + justfile + devcontainer working. Windows bootstrap missing.
9. Security workflows: Scorecard + SBOM workflows present. Secret scanning not yet added.
10. Package maintenance: No packages published. Package dry-run scaffolding exists.

Quarterly outputs pending: sota-gap-analysis.md, refreshed CI/toolchain matrix, red-team report addendum.

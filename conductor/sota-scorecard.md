# SOTA Readiness Scorecard

| Dimension | Current target | Release gate |
|---|---|---|
| Deterministic scheduler | Fixed-tick SimTime, stable event order | Property tests + replay fixtures |
| DES/ABM parity | Event-first model, ECS entities | Hybrid example passes |
| Cross-language compatibility | C ABI + generator facades | Two bindings pass conformance before beta |
| Python support | 3.10-3.14, optional 3.14 free-threaded checks | Wheels/smoke tests |
| C# support | .NET 10 stable, .NET 11 preview/GA lane | NuGet dry run and SafeHandle tests |
| Telemetry | Arrow IPC/Parquet outputs | Host languages read event logs |
| Reproducibility | seed manifest + scenario manifest + replay | At least DES and ABM examples emit manifests |
| V&V | statistical checks, sensitivity analysis | One validation tutorial |
| Documentation | GitHub Pages + API docs + tutorials | Docs build and link-check pass |
| Benchmarks | fair benchmark suite | Published reproducibility instructions |
| Supply chain | SBOM, attestations, Scorecard, Best Practices | Release checklist gate |
| Community | model zoo, good-first issues, governance | Contributor onboarding page |
| Publishing | staged registries | Test registry/dry-run before production publish |
| Maintenance | deprecation and compatibility policy | Public compatibility table |
| Interoperability | standards review | mapping docs published |

## Scoring rubric

```text
0 = not planned
1 = planned but no artifact
2 = artifact exists
3 = artifact tested or reviewed
4 = included in CI/release gates
5 = proven in public release
```

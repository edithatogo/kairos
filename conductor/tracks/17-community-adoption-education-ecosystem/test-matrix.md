# Test Matrix: Track 17 Community Adoption, Education & Ecosystem

| Check | Validation command | Required by alpha | Required by beta | Required by 1.0 |
|---|---|---:|---:|---:|
| Discovery page renders and shows a clear install path, quickstart path, and contributor entry points | `just docs-build && test -f website/build/index.html` | yes | yes | yes |
| `website/src/index.md` names maturity labels for alpha, beta, and stable surfaces | `rg -n "alpha|beta|stable|experimental|preview" website/src/index.md conductor/tracks/17-community-adoption-education-ecosystem/community-plan.md` | yes | yes | yes |
| `conductor/package-catalog.md` points at the current docs, package, and release surfaces | `rg -n "docs/community/|docs/release/|docs/benchmarks/|docs/research/|docs/trustworthy-simulation/" conductor/package-catalog.md website/src/index.md` | yes | yes | yes |
| Contributor UX has visible contribution, issue, and triage entry points | `rg -n "contributing|issue templates|good first issue|help wanted|triage" conductor/tracks/17-community-adoption-education-ecosystem/community-plan.md website/src/index.md` | yes | yes | yes |
| Docs link checks pass | `just check-docs` | yes | yes | yes |
| Docs build smoke test passes | `cd website && npm ci && npm run build` | yes | yes | yes |
| Release-gate adoption criteria are explicit for public beta | `rg -n "public beta|runnable examples|contribution guide|security policy|benchmark harness|conformance fixture" conductor/tracks/17-community-adoption-education-ecosystem/handoff.md conductor/tracks/17-community-adoption-education-ecosystem/community-plan.md` | no | yes | yes |
| Community claims match tracked repo artifacts | `rg -n "community|adoption|education|package catalog|maturity labels" conductor/package-catalog.md conductor/tracks/17-community-adoption-education-ecosystem/spec.md website/src/index.md` | yes | yes | yes |

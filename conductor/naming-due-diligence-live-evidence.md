# KairoECS Naming Due Diligence Live Evidence

Review date: 2026-05-07
Reviewer: Codex

This note records the live registry and public-identity searches run while updating `conductor/naming-due-diligence.md`.

## Registry and public-identity searches

| Review date | Reviewer | Surface | Query/source | Exact name checked | Observed result | Decision impact |
|---|---|---|---|---|---|---|
| 2026-05-07 | Codex | crates.io | Live search restricted to `crates.io` for `kairo-ecs` | `kairo-ecs` | No exact match surfaced in live search. | Keep registry name reserved for later publication review. |
| 2026-05-07 | Codex | PyPI | Live search restricted to `pypi.org/project` for `kairo-ecs` | `kairo-ecs` | No exact match surfaced in live search. | Keep Python distribution decision under review. |
| 2026-05-07 | Codex | npm | Live search restricted to `npmjs.com/package` for `@kairo-ecs/typescript` | `@kairo-ecs/typescript` | No exact match surfaced in live search. | Keep npm scope/package decision under review. |
| 2026-05-07 | Codex | NuGet | Live search restricted to `nuget.org/packages` for `Kairo.ECS` | `Kairo.ECS` | No exact match surfaced in live search. | Keep NuGet package decision under review. |
| 2026-05-07 | Codex | Julia registry | Live search restricted to Julia registry sources for `KairoECS` | `KairoECS` | No exact match surfaced in live search. | Keep Julia package decision under review. |
| 2026-05-07 | Codex | R release channels | Live search restricted to CRAN and R-universe sources for `kairoECS` | `kairoECS` | No exact match surfaced in live search. | Keep R package decision under review. |
| 2026-05-07 | Codex | pkg.go.dev | Live search restricted to `pkg.go.dev` for `github.com/edithatogo/kairos/bindings/go` | `github.com/edithatogo/kairos/bindings/go` | No exact match surfaced in live search. | Go module path remains a local declaration, not a verified public release decision. |
| 2026-05-07 | Codex | GitHub | Live search restricted to `github.com` for `edithatogo/kairos` | `edithatogo/kairos` | No exact match surfaced in live search. | Public release repo decision still needs maintainer sign-off. |
| 2026-05-07 | Codex | Domains | ICANN/lookup-oriented live search for `kairo-ecs.dev` and `kairo-ecs.org` | `kairo-ecs.dev`, `kairo-ecs.org` | No exact match surfaced in live search. | Domain acquisition remains blocked pending registrar review. |
| 2026-05-07 | Codex | Trademark / common law | Live search for `KairoECS`, `Kairo ECS`, and `kairo-ecs` | `KairoECS`, `Kairo ECS`, `kairo-ecs` | No exact-match result surfaced; close-variant `KAIRO` marks did appear in US trademark search results. | Legal/trademark review remains required before public release. |

## Notes

- The search pass was limited to live public web/registry lookups and did not fabricate availability.
- The results above are enough to document the current diligence pass, but they do not replace legal clearance or registrar confirmation.
- The repository still treats public publishing as blocked until the maintainer records the remaining surface decisions and legal/trademark advice.

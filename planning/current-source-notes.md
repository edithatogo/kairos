# Current Source Notes to Verify Before Implementation

These notes reflect live checks performed while preparing this pack on 2026-05-04. Re-verify before implementation and every release because registry policies, toolchains, and Conductor behavior can change.

## Conductor

- The Conductor extension describes a context-driven flow of `Context -> Spec & Plan -> Implement`.
- `/conductor:setup` generates shared artifacts including `conductor/product.md`, `conductor/product-guidelines.md`, `conductor/tech-stack.md`, `conductor/workflow.md`, `conductor/code_styleguides/`, and `conductor/tracks.md`.
- `/conductor:newTrack` generates per-track `spec.md`, `plan.md`, and `metadata.json` under `conductor/tracks/<track_id>/`.
- Source checked: https://github.com/gemini-cli-extensions/conductor

## Runtime version coverage

- Python 3.14 is a stable release series and has 2026 maintenance releases. Python binding coverage should include CPython 3.10, 3.11, 3.12, 3.13, and 3.14.
- Python 3.14 includes official free-threaded Python support; add a free-threaded smoke lane where CI runner/tooling support exists.
- .NET 10 is listed by Microsoft as an LTS supported release, supported until November 2028.
- .NET 11 is in preview as of this pack and final release is expected in November 2026; include a required planning lane but keep stable NuGet promises GA-gated.
- Sources checked: Python.org version docs and Microsoft Learn .NET release/versioning docs.

## Publishing and delivery

- PyPI Trusted Publishing uses OIDC and avoids long-lived API tokens for supported CI providers.
- npm provenance can expose where/how packages were built and published; use it for npm releases when feasible.
- GitHub Pages deployment via Actions requires `pages: write` and `id-token: write` permissions in the deploy job.
- crates.io publishes are permanent for a version: versions cannot be overwritten and code cannot be deleted; dry runs and package contents inspection are mandatory.

## Supply chain and trust

- OpenSSF Scorecard has an official GitHub Action and should run on a schedule for public repositories.
- OpenSSF Best Practices Badge is a no-cost FLOSS self-certification process and should be targeted before 1.0.
- GitHub artifact attestations provide signed provenance/integrity claims for artifacts built by GitHub Actions.
- SLSA provenance is the recommended predicate for describing how artifacts were produced.
- `CITATION.cff` is human- and machine-readable citation metadata supported by GitHub and Zenodo and should be present from the first public release.

## Additional naming observations

- The earlier bare `Kairos` naming option has known package ecosystem collisions/signals and should not be used for public package names without legal and registry review.
- The KairoECS decision intentionally uses distinctive public names such as `kairo-ecs`, `@kairo-ecs/core`, `Kairo.ECS`, `kairoECS`, and `KairoECS.jl`.
- Exact availability of those names must still be re-checked immediately before public publishing because registry state can change.

See `planning/source-verification.md` and `conductor/naming-due-diligence.md` for the release-planning implications.

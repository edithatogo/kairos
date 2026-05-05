# Source Verification Notes

Prepared: 2026-05-04.

Re-run before implementation and release.

## Conductor

- Source: https://github.com/gemini-cli-extensions/conductor
- Checked facts:
  - Conductor frames the lifecycle as `Context -> Spec & Plan -> Implement`.
  - `/conductor:setup` generates project context artifacts such as product, product guidelines, tech stack, workflow, code style guides, and tracks registry.
  - `/conductor:newTrack` generates per-track `spec.md`, `plan.md`, and metadata under `conductor/tracks/<track_id>/`.

## Python

- Source: https://www.python.org/downloads/latest/python3/
- Checked facts:
  - Python 3.14 has current 2026 maintenance releases.
  - Python 3.14 includes officially supported free-threaded Python.
- Planning implication:
  - Python support matrix remains CPython 3.10-3.14.
  - Add a 3.14 free-threaded smoke lane once packaging/tooling supports it.

## .NET

- Source: https://dotnet.microsoft.com/en-us/platform/support/policy
- Source: https://learn.microsoft.com/en-us/dotnet/core/whats-new/dotnet-11/overview
- Checked facts:
  - .NET 10 is LTS and supported until November 14, 2028.
  - .NET 11 is preview as of May 2026 and final release is expected in November 2026.
- Planning implication:
  - `net10.0` is the stable C# lane.
  - `net11.0` is planned/preview coverage, allowed-failure until GA.

## PyPI

- Source: https://docs.pypi.org/trusted-publishers/
- Source: https://docs.pypi.org/trusted-publishers/using-a-publisher/
- Checked facts:
  - Trusted Publishing uses OIDC and avoids manually managed long-lived API tokens.
  - GitHub Actions publishing needs `id-token: write`.
- Planning implication:
  - Use TestPyPI first, then PyPI Trusted Publishing for releases where possible.

## npm

- Source: https://docs.npmjs.com/generating-provenance-statements
- Source: https://docs.npmjs.com/viewing-package-provenance
- Checked facts:
  - npm provenance can show where/how packages were built and published.
  - Publishing with provenance uses `npm publish --provenance` on supported cloud CI and requires `id-token: write`.
  - `npm audit signatures` can verify registry signatures/provenance attestations.
- Planning implication:
  - TypeScript/Wasm release workflow should use npm provenance where feasible.

## NuGet

- Source: https://learn.microsoft.com/en-us/nuget/nuget-org/trusted-publishing
- Checked facts:
  - NuGet Trusted Publishing uses GitHub Actions OIDC to obtain short-lived credentials and is rolling out gradually.
- Planning implication:
  - Prefer NuGet Trusted Publishing when available; otherwise use scoped secrets with protected environments.

## GitHub Pages

- Source: https://github.com/actions/deploy-pages
- Checked facts:
  - The deploy job needs `pages: write` and `id-token: write` permissions.
  - `actions/deploy-pages` deploys an uploaded Pages artifact.
- Planning implication:
  - Docs workflow should use build/upload/deploy jobs with least permissions.

## crates.io

- Source: https://doc.rust-lang.org/cargo/reference/publishing.html
- Checked facts:
  - Publishing a crate version is permanent; the version cannot be overwritten and code cannot be deleted.
- Planning implication:
  - Use `cargo package --list`, dry runs, and release approvals before any publish.

## R-universe

- Source: https://docs.r-universe.dev/
- Source: https://docs.r-universe.dev/browse/get-started.html
- Checked facts:
  - R-universe supports R package discovery, publishing, and continuous integration-like builds from Git repositories.
  - R-universe does not vet packages; quality policy remains with the universe owner.
- Planning implication:
  - Use R-universe before CRAN, but do not treat it as external quality review.

## Julia registration

- Source: https://help.juliahub.com/juliahub/stable/registering/
- Checked facts:
  - Registrator creates registration pull requests in Julia's General registry.
  - Packages need valid `name`, `uuid`, and `version` fields.
- Planning implication:
  - Use dev/local registry before General registration; add Registrator/TagBot later.

## Citation and archival

- Source: https://help.zenodo.org/docs/github/describe-software/citation-file/
- Source: https://help.zenodo.org/docs/github/describe-software/zenodo-json/
- Checked facts:
  - `CITATION.cff` helps GitHub display citation suggestions.
  - If `.zenodo.json` and `CITATION.cff` both exist, Zenodo uses `.zenodo.json` for GitHub release archiving.
- Planning implication:
  - Keep both, but make `.zenodo.json` authoritative for Zenodo-specific metadata.

## OpenSSF and Sigstore

- Source: https://openssf.org/projects/best-practices-badge/
- Source: https://github.com/ossf/scorecard-action
- Source: https://docs.sigstore.dev/cosign/signing/overview/
- Checked facts:
  - OpenSSF Best Practices Badge is a no-cost FLOSS self-certification process.
  - Scorecard has an official GitHub Action.
  - Sigstore/Cosign supports identity-based/keyless signing using OIDC, short-lived certificates, and transparency logs.
- Planning implication:
  - Add Scorecard, OpenSSF badge checklist, SBOMs, signatures/attestations, and release provenance before 1.0.

## Naming verification requirements

The previous bare `Kairos` option has known ecosystem collision/signals, including an existing PyPI `kairos` package and a NuGet `Kairos.Net` package. The KairoECS naming decision avoids bare `kairos`/`kairo` registry names and prefers the distinctive `kairo-ecs` family.

Before publishing, verify these exact names live:

```text
crates.io: kairo-ecs, kairo-ecs-core, kairo-ecs-state, kairo-ecs-ffi, kairo-ecs-arrow
PyPI: kairo-ecs
npm: @kairo-ecs/core and @kairo-ecs scope
NuGet: Kairo.ECS
R release channel: kairoECS
Julia General: KairoECS.jl
GitHub: kairo-ecs org/repo
Go module path: github.com/<org>/kairo-ecs
Docs domain: kairo-ecs.dev or fallback
```

Planning implication: KairoECS can proceed as the working brand, but public registry publication remains blocked until the exact name-availability and trademark/common-law checklist is complete.

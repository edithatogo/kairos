# Contributor Onboarding

Welcome to KairoECS. The fastest useful contribution is one that names the track it belongs to, changes only the owned surface, and includes the check that proves the change.

## First contribution path

1. Read the root `README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, and `SECURITY.md`.
2. Read `conductor/track-map.md` and the relevant `conductor/tracks/<track>/spec.md`.
3. Pick an issue labelled `good first issue`, `help wanted`, `kind:docs`, or the matching `track:<area>` label.
4. Confirm the owned paths before editing. If ownership is unclear, ask in the issue before opening a PR.
5. Create a branch named `track/<nn>-<slug>` for track work or `docs/<short-topic>` for docs-only work.
6. Make the smallest change that satisfies the issue and update any affected docs or `handoff.md`.
7. Run the local check named by the issue, track `test-matrix.md`, or docs page.
8. Open a PR with the changed files, commands run, and any skipped checks.

## Local docs checks

Use the narrowest check that covers your change:

| Change | Minimum check |
|---|---|
| Local contributor setup | `just dev-validate` |
| Community docs only | `rg -n "onboarding-docs|Contributor Onboarding|Model Zoo|Community Adoption" docs/community conductor/tracks/17-community-adoption-education-ecosystem` |
| Docs site entry point | `just check-docs` |
| Website build or navigation | `just docs-build` |
| Conductor track evidence | `just validate-track-docs` |

## Good first issue shape

A good first issue should include:

- expected file or directory
- acceptance check
- maturity label if it changes a public claim
- owner or reviewing subagent
- clear non-goals

Avoid issues that ask a new contributor to design a public API, publish packages, change the C ABI, or add release claims without evidence.

## PR checklist

- Scope stays inside the owned paths named by the issue or track.
- Links use repo-relative Markdown links where possible.
- New examples appear in `examples/model-zoo/model-zoo.yaml` and `docs/community/model-zoo.md`.
- Public claims use one of the maturity labels from [model-zoo.md](model-zoo.md).
- The PR description lists commands run and any waived check with a reason.

## Getting help

Use the public governance and conduct paths before private escalation:

- Governance: [governance.md](governance.md)
- Roadmap and maturity stages: [roadmap.md](roadmap.md)
- Security contact path: root `SECURITY.md`
- Maintainer and review expectations: `MAINTAINERS.md` and `governance/`

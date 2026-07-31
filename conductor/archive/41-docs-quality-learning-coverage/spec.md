# 41 Documentation Platform, Quality Gates & Learning Coverage -- spec.md

## Mission

Close the remaining repo-wide polish gaps in one track: make CI, linting,
formatting, validation, typing, and docstring policy strict where the project
already has a concrete surface; migrate the public documentation stack to the
Astro/Starlight roadmap; and finish the learning-coverage inventory for the
supported language and example surfaces so tutorials, examples, and notebook
artifacts are discoverable by intent instead of by accident.

## Primary subagent

```text
docs-agent + ci-agent + community-agent
```

## Dependencies

```text
Track 06-11 (binding surfaces and language-specific validation),
Track 12 (shared conformance and notebook/runtime fixtures),
Track 13 (CI/CD, code quality, and supply chain),
Track 14 (documentation site),
Track 17 (community onboarding and education),
Track 23 (model zoo and starter kits),
Track 24 (playground and demo surfaces),
Track 27 (developer experience and reproducible environments),
Track 30 (toolchain/version matrix).
```

## Owned paths

```text
.github/workflows/,
docs/,
examples/,
notebooks/,
templates/website/,
website/,
scripts/validation/
```

## Blocked paths

```text
crates/ -- owned by implementation tracks and not modified here without handoff.
bindings/ -- owned by language binding tracks and not modified here without handoff.
cloud/ -- owned by Track 39.
hpc/ -- owned by Track 39.
```

## Parallel-safe with

```text
Most of this track is parallel-safe once the coverage matrix is agreed:
CI strictness, docs-stack migration, notebook validation, and language/example
inventory updates can move in separate branches with clear handoff notes.
```

## Inputs

- Current GitHub Actions workflow set and quality gates.
- Current website implementation and docs tree.
- Current tutorial, example, and notebook inventory across the supported
  languages.
- The docs roadmap entry that calls for Astro/Starlight.
- Binding and example surfaces from Tracks 06-11, 23, and 24.

## Outputs

- A strictness policy for the repo's existing CI surfaces, including warnings-
  as-errors where the surface is already concrete and documented skips only
  where the environment truly blocks execution.
- A docs-platform migration plan or implementation that closes the custom
  website vs Astro/Starlight gap without losing current docs coverage.
- A language-by-language learning coverage matrix covering tutorials,
  examples, and notebook assets, with explicit exclusions where notebooks are
  not the right medium.
- Updated docs navigation so tutorials, examples, and notebooks are surfaced
  from the same entry points.
- Validation notes that keep notebook/tutorial inventories in sync with the
  docs site and community pages.

## Acceptance criteria

- Trusted CI paths fail on warnings or unsupported drift for the concrete
  surfaces they own, while forked/no-secret scenarios remain explicitly
  documented exceptions.
- The public docs stack either runs on Astro/Starlight or carries a clearly
  documented parity list that closes every current docs-site gap.
- Every supported language has a documented learning artifact path:
  tutorial, example, or notebook, with a matrix that explains any deliberate
  notebook exclusion.
- Notebook and tutorial inventories stay synchronized with the docs site and
  community onboarding pages.
- `handoff.md` is completed before merge.

## Release implications

This track is not a hard release gate by itself, but it directly affects user
trust, discoverability, and the credibility of strict CI claims. It should be
completed before any public "fully documented" or "fully strict" claim is made.

## Non-goals

- This track does NOT reimplement the binding surfaces themselves.
- This track does NOT add new core engine features.
- This track does NOT force notebooks for a language where a notebook is not
  the best delivery medium; instead it documents the chosen learning artifact.

## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be
listed in `test-matrix.md`.

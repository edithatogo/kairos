# Contributing to KairoECS

Thanks for considering a contribution. KairoECS is intentionally contract-first because it spans Rust plus Python, R, Julia, TypeScript, C#, and Go.

## Start here

1. Read `README.md`.
2. Read `conductor/workflow.md`.
3. Pick a track in `conductor/track-map.md`.
4. Check owned paths in `conductor/subagents.md`.
5. Use the API review template for any public API/ABI/schema change.

## Track-aware first contribution path

Use this path for a first KairoECS contribution:

1. Pick an issue labelled `good first issue`, `help wanted`, `kind:docs`, `model-zoo`, or the relevant `track:<area>` label.
2. Read the matching track under `conductor/tracks/<nn>-*/`, especially `spec.md`, `plan.md`, `test-matrix.md`, and `handoff.md`.
3. Confirm the owned paths in `conductor/tracks.yaml` before editing.
4. Keep the patch narrow and update the affected handoff or test matrix when the check surface changes.
5. Run the smallest validator that proves the change. Community and onboarding updates must keep the `onboarding-docs` gate green:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\17-community-adoption-education-ecosystem\validate-community-onboarding.ps1
   ```

6. In the PR, list changed files, commands run, skipped or blocked checks, and any maturity-label impact.

The public onboarding detail lives at `docs/community/contributor-onboarding.md`; it links governance, conduct, security, issue, review, and model-zoo paths for first-time contributors.

## Required checks before PR

```bash
just fmt
just lint
just test
```

If the full environment is not ready yet, run the relevant subset and explain what could not be run.

## Public API changes

Public Rust APIs, C ABI signatures, Arrow schemas, and host-language APIs require:

- ADR
- conformance fixture update
- docs update
- compatibility assessment
- red-team objection response

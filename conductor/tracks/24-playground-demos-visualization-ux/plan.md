# Track 24 Plan: Playground, Demos & Visualization UX

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- Review the docs site, example, and visualization surfaces already present in the repository.
- Identify where this track consumes `website/`, any future playground assets, and the release docs.
- Open an ADR if a demo would imply a new public product claim or support promise.

### Task 0.2 — Define owned artifacts
- Keep the work centered on playground pages, demo copy, visualization assets, and usage notes.
- Add owner/subagent to `conductor/subagents.md` if missing.
- Add checks where the demo can be exercised locally.

## Phase 1 — Minimum viable public artifact

### Task 1.1 — Create the first usable version
- [x] Produce the smallest playable demo page or visualization note that helps a user understand the current repo state.
- [x] Use a real KairoECS example path, screenshot target, or fixture reference that can be exercised in the browser.

Evidence: `website/playground/index.html` renders `website/playground/headless-snapshot.json`, anchored to `examples/viz/headless-snapshot/src/main.rs`.

### Task 1.2 — Add review criteria
- Add red-team prompts for misleading motion, broken layouts, and demos that overstate the implementation.
- Add devil's advocate objections about whether the UX helps users inspect the system rather than just decorate it.
- Add measurable acceptance criteria for layout stability and asset presence.

## Phase 2 — Automation and validation

### Task 2.1 — Wire into CI where possible
- [x] Add docs linting, asset-existence checks, and smoke tests for any local preview or demo page.
- Use path guards for playground assets that are not created yet.

Evidence: `node website/scripts/smoke-playground.mjs` validates the page assets, fixture schema, source anchor, summary counts, and bounds.

### Task 2.2 — Connect to release gates
- Define what demo and visualization evidence is required before alpha, beta, RC, and 1.0 claims.
- Add the playground checks to `conductor/delivery-readiness-checklist.md`.

## Phase 3 — Cross-track integration

### Task 3.1 — Handoff to dependent tracks
- Document exactly what other subagents can rely on: asset locations, demo targets, and preview entry points.
- Provide example assets or fixture references rather than prose-only handoffs.

### Task 3.2 — Add community-facing documentation
- [x] Ensure the docs site has a page explaining how to use the playground and read the visuals.
- [x] Link from the docs index or contributor guide where the page is easy to find.

Evidence: `docs/community/playground.md` names the implemented slice and `docs/playground/headless-snapshot.md` records the local commands.

## Phase 4 — Closeout

### Task 4.1 — Run quality gates
- Check markdown links.
- Validate the demo page renders without overlapping content.
- Run the local preview or smoke workflow.

### Task 4.2 — Update risk register
- Move resolved risks to mitigated.
- Keep unresolved layout or asset risks as blockers until the demo is stable.

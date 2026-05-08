# Track 18 Plan: Comparative Benchmarks & Reproducibility

## Phase 0 - Contract alignment

### Task 0.1 - Read the benchmark sources
- Read `benches/benchmark-plan.md`, `conformance/fixtures/manifest.json`, and `.github/workflows/benchmark-smoke.yml` together so the docs describe the real measurement path.
- Anchor the public benchmark story to the ready fixture IDs:
  - `scheduler_ordering_v1`
  - `scheduler_cancellation_v1`
  - `rng_reproducibility_v1`
- Open an ADR only if the page needs a new metric definition, baseline, or comparison claim.

### Task 0.2 - Define the evidence boundary
- Keep the work centered on benchmark definitions, reproducibility notes, fixture manifests, and comparison criteria.
- Treat `benchmarks/benchmark-smoke.yml` as the local smoke gate only if it matches the actual workflow contract in `.github/workflows/benchmark-smoke.yml`.
- Keep the track focused on evidence that can be replayed from committed fixtures and benchmark plan text.

## Phase 1 - Minimum viable public artifact

### Task 1.1 - Describe one concrete comparison path
- Produce the smallest note that explains measurement setup, seed control, fixture source, and expected comparison output.
- Use a real fixture or benchmark target from the ready set, not a synthetic placeholder.
- State what the benchmark smoke workflow checks and what it does not prove.

### Task 1.2 - Add review criteria
- Add red-team prompts for unfair comparisons, unstated assumptions, and unstable host environments.
- Add devil's advocate objections about whether the benchmark is reproducible on another machine.
- Add measurable acceptance criteria for stable seeds, recorded inputs, fixture IDs, and repeatable output.

## Phase 2 - Automation and validation

### Task 2.1 - Wire to the smoke gate
- Keep the workflow contract aligned to `.github/workflows/benchmark-smoke.yml`, which checks repo shape and `cargo bench --workspace --no-run`.
- Add docs linting or manifest checks only where they reinforce the same benchmark story.
- Use path guards for any future benchmark targets that are not created yet.

### Task 2.2 - Connect to reproducibility evidence
- Define what evidence is required before alpha, beta, RC, and 1.0 comparison claims are allowed.
- Keep the evidence rooted in `benches/benchmark-plan.md`, the fixture manifest, and the smoke workflow path.

## Phase 3 - Cross-track integration

### Task 3.1 - Handoff to dependent tracks
- Document exactly what other subagents can rely on: seed control, fixture IDs, smoke workflow path, and comparison criteria.
- Provide the committed fixture IDs and example benchmark targets rather than prose-only handoffs.

### Task 3.2 - Add community-facing documentation
- Ensure the docs site has a page explaining how to reproduce the comparison.
- Link from the benchmark page, model zoo, or release notes as appropriate.

## Phase 4 - Closeout

### Task 4.1 - Run quality gates
- Check markdown links.
- Validate the benchmark plan renders cleanly.
- Run the benchmark smoke workflow or fixture validation path locally.

### Task 4.2 - Update risk register
- Move resolved risks to mitigated.
- Keep unresolved reproducibility drift as a release blocker until the comparison is repeatable from the committed fixtures.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update the track registry/status surfaces: `conductor/tracks.yaml` (authoritative machine-readable registry), `conductor/tracks.md` (human index), `conductor/phase-closeout.yaml` (review ledger), `conductor/status.md` (narrative status), and `conductor/implementation-readiness.md` or `conductor/track-map.md` when readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.
# Track 22 Plan: Experiment Runner & Scenario Management

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- Review the runner, fixture, and reproducibility surfaces that support scenario management.
- Identify where this track consumes `conformance/fixtures/manifest.json`, `benches/benchmark-plan.md`, and the docs build path.
- Open an ADR if a scenario runner would alter a public execution or replay contract.

### Task 0.2 — Define owned artifacts
- Keep the work centered on scenario manifests, run instructions, replay notes, and example outputs.
- Add owner/subagent to `conductor/subagents.md` if missing.
- Add smoke checks for scenarios that can already be executed locally.

## Phase 1 — Minimum viable public artifact

### Task 1.1 — Create the first usable version
- Produce the smallest runner note that explains how to execute, replay, and compare a named scenario.
- Use one real KairoECS scenario or fixture ID that the runner can execute and replay.

### Task 1.2 — Add review criteria
- Add red-team prompts for accidental non-determinism, hidden inputs, and state leakage between runs.
- Add devil's advocate objections about whether the runner can be trusted across machines.
- Add measurable acceptance criteria for repeatable scenario output and replay metadata.

## Phase 2 — Automation and validation

### Task 2.1 — Wire into CI where possible
- Add docs linting, scenario manifest validation, and replay smoke checks where possible.
- Use path guards for any future runner outputs that are not created yet.

### Task 2.2 — Connect to release gates
- Define what scenario and replay evidence is needed before alpha, beta, RC, and 1.0 claims.
- Add the runner checks to `conductor/delivery-readiness-checklist.md`.

## Phase 3 — Cross-track integration

### Task 3.1 — Handoff to dependent tracks
- Document exactly what other subagents can rely on: scenario IDs, replay inputs, and expected output shape.
- Provide fixture references and sample commands rather than prose-only handoffs.

### Task 3.2 — Add community-facing documentation
- Ensure the docs site has a page explaining how to run and replay scenarios.
- Link the CLI reference from the docs index or contributor guide where the runner is discoverable.

## Phase 4 — Closeout

### Task 4.1 — Run quality gates
- Check markdown links.
- Validate the runner notes render cleanly.
- Run the scenario smoke or replay workflow.

### Task 4.2 — Update risk register
- Move resolved risks to mitigated.
- Promote unresolved replay or determinism risks to release blockers.
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
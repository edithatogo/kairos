# Track 29 Plan: Wave Manager & Execution Gatekeeper

## Phase 0 - Scope lock

### Task 0.1 - Read the current track inventory
- Parse `conductor/tracks.yaml` to extract all track IDs, statuses, and dependencies.
- Cross-reference with `conductor/track-map.md` for the full DAG.
- Identify any tracks with circular or underspecified dependencies and escalate.
- Confirm that wave membership can be derived from the dependency graph without relying on a fixed track count or fixed maximum wave.

### Task 0.2 - Lock the owned surface
- Keep all new work to `conductor/tracks/29-wave-manager-execution-gatekeeper/`.
- Write gate definitions into `conductor/gates/` only.
- Do not modify track YAML schema, status vocabulary, or CI workflows.

## Phase 1 - Define wave policy

### Task 1.1 - Map tracks to waves
- Derive wave membership from the dependency graph:
  - Wave 0: Tracks with no dependencies (00).
  - Wave 1: Tracks depending only on Wave-0 tracks.
  - Wave 2: Tracks depending on Wave-1 or lower.
  - Wave 3: Tracks depending on Wave-2 or lower.
  - Wave 4: Tracks depending on Wave-3 or lower.
  - Wave N: Tracks depending on Wave-(N-1) or lower.
- Publish the mapping in `conductor/wave-policy.md`.

### Task 1.2 - Define gating rules
- Specify that a track cannot transition from "Planned" to "In Progress" unless all its `depends_on` tracks are "Done".
- Define the exception path: maintainer override via ADR with documented rationale.
- Specify that transitive dependencies (deps of deps) must also be satisfied.

### Task 1.3 - Define critical-path heatmap
- Identify tracks that gate the most downstream tracks.
- Publish a ranked list with dependency counts.

## Phase 2 - Implement gate logic

### Task 2.1 - Write wave-progression-check gate
- Read `conductor/tracks.yaml` for current status.
- For each track not yet "Done", verify all `depends_on` entries are "Done".
- Return pass/fail with track ID and missing dependency details.

### Task 2.2 - Write dependency-closure-check gate
- Compute the transitive closure of each track's `depends_on`.
- Verify all transitive dependencies are "Done" before allowing advancement.
- Flag any dependency cycle.

### Task 2.3 - Wire gates into quality-gates.md
- Add `wave-progression-check` and `dependency-closure-check` to `conductor/quality-gates.md`.
- Document expected failure modes and resolution paths.

## Phase 3 - Handoff and release planning

### Task 3.1 - Prepare maintainer notes
- Document the exception override procedure.
- List which tracks are currently gating which downstream tracks.
- Provide the critical-path heatmap for release planning.

### Task 3.2 - Cross-track communication
- Hand off wave-policy enforcement to Track 13 (CI/CD) for workflow integration.
- Hand off gate definitions to Track 15 (Packaging) for release gate wiring.
- Notify all track owners of their assigned wave and what blocks their advancement.

## Phase 4 - Closeout

### Task 4.1 - Validate wave policy consistency
- Run both gates against the current track status snapshot.
- Run the report-only mode to capture derived wave membership and critical-path heatmap.
- Verify no false positives (tracks blocked when deps are actually "Done").
- Verify no false negatives (tracks allowed when deps are not "Done").

### Task 4.2 - Update the risk register
- Mark resolved risks as mitigated.
- Escalate any wave-policy violations that cannot be resolved within the current status snapshot.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update `conductor/phase-closeout.yaml` with review outcome, accepted fixes, validation commands, cleanup state, commit SHA or blocker, pushed ref, and next-phase decision.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.
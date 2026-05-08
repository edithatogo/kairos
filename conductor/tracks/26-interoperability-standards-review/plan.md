# Track 26 Plan: Interoperability Standards Review

## Phase 0 — Contract alignment

### Task 0.1 — Read existing contracts
- Review the interoperability claims already implied by `conductor/interoperability-standards.md`, `conductor/compatibility-promise.md`, `conductor/testing-strategy.md`, `conductor/experiment-runner.md`, `conductor/trustworthy-simulation.md`, and the docs pages under `docs/trustworthy-simulation/`.
- Identify where this track consumes the existing standards vocabulary: DEVS, FMI/FMU, SBML, CellML, OpenTelemetry semantic conventions, Arrow C Data Interface, Arrow IPC, Parquet, and the DES/ABM ecosystem mappings already named in the repo.
- Open an ADR if a standards review would change a public compatibility claim.

### Task 0.2 — Define owned artifacts
- Keep the work centered on standards inventories, mapping notes, and cross-ecosystem compatibility summaries.
- Distinguish supported, partial, deferred, and explicitly unsupported mappings.
- Make the docs page tell a new reader which interoperability claims are conceptual, which are data-exchange oriented, and which are release-impacting.
- Add owner/subagent to `conductor/subagents.md` if missing.
- Add checks where the standards inventory can be validated locally.

## Phase 1 — Minimum viable public artifact

### Task 1.1 — Create the first usable version
- Produce the smallest standards review note that maps KairoECS concepts to outside formats or ecosystems.
- Use concrete mappings such as:
  - DEVS concepts -> KairoECS event ordering, scheduler, and replay vocabulary.
  - Arrow C Data Interface / Arrow IPC / Parquet -> KairoECS telemetry and event-log surfaces.
  - OpenTelemetry semantic conventions -> trace/log naming guidance only.
  - FMI/FMU, SBML, and CellML -> future bridge targets that are not yet implemented.
- Tie each mapping to a comparison target that already exists in the repo, such as `conductor/experiment-runner.md`, `docs/trustworthy-simulation/replay-and-seeds.md`, or the Arrow schema work.

Worker 3 evidence note: `docs/interoperability/standards-review.md` now carries the concrete standards mapping with required labels for DEVS, FMI/FMU, SBML, CellML, OpenTelemetry semantic conventions, Arrow C Data Interface, Arrow IPC, and Parquet. `conductor/interoperability-standards.md` mirrors the current labels for conductor-facing readers.

### Task 1.2 — Add review criteria
- Add red-team prompts for false interoperability claims, partial mappings, and ambiguous terminology.
- Add devil's advocate objections about whether the standard mapping is useful in practice.
- Add measurable acceptance criteria for supported and unsupported mappings:
  - supported mappings name the exact standard and the exact KairoECS surface;
  - partial mappings state the missing behavior;
  - unsupported mappings are explicitly labeled as deferred.

## Phase 2 — Automation and validation

### Task 2.1 — Wire into CI where possible
- Add docs linting, inventory checks, and smoke validation for any generated mapping tables.
- Use path guards for future interoperability artifacts that are not created yet.
- Keep the validation local to this track's docs and the docs page it owns.

Worker 3 evidence note: local validation is available at `conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1`. It checks the eight required standards, the supported/partial/deferred/unsupported vocabulary, evidence citations, release-impacting assertions, and exactly eight primary mapping rows.

### Task 2.2 — Connect to release gates
- Define what standards-review evidence is required before alpha, beta, RC, and 1.0 claims.
- Record the release-impacting assertions in the docs page itself: Arrow schema changes, semantic-convention drift, and any claim of external runtime interoperability must be reviewable before release language is used.

## Phase 3 — Cross-track integration

### Task 3.1 — Handoff to dependent tracks
- Document exactly what other subagents can rely on: supported mappings, known gaps, and vocabulary choices.
- Provide concrete mapping tables rather than prose-only handoffs.
- Include the exact standards names and the exact KairoECS targets for comparison.

### Task 3.2 — Add community-facing documentation
- Ensure the docs site has a page explaining how interoperability decisions were made.
- Link from the package catalog or contributor guide as appropriate.
- Keep the page honest about what is conceptual guidance versus what is a supported interchange format.

## Phase 4 — Closeout

### Task 4.1 — Run quality gates
- Check markdown links.
- Validate the mapping tables render cleanly.
- Run the docs build or smoke workflow.
- Confirm the docs page and track docs mention the same standards and gap labels.

### Task 4.2 — Update risk register
- Move resolved risks to mitigated.
- Keep unresolved interoperability gaps as blockers until the mapping is explicit.

Worker 3 evidence note: the risk register now includes explicit risks for overstated Arrow IPC/Parquet support, FMI/FMU scaffold overclaiming, OpenTelemetry exporter overclaiming, and unsupported ecosystem comparisons becoming compatibility claims.
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
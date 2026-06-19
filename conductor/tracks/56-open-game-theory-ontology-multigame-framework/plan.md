# Track 56 Plan: Open Game Theory Ontology and Multi-Game Framework

## Phase 0 - Track creation

- Task 0.1: Create the Track 56 Conductor artifact set and registry entries.
- Task 0.2: Add central quality gates for ontology ingestion, graph-relations
  feature isolation, pointer-free topology, and game solver validation.
- Task 0.3: Record release-claim boundaries in status, readiness, track map,
  SOTA scorecard, and changelog surfaces.

## Phase 1 - Subrepo and ontology ingestion

- Task 1.1: Add a failing test or validator proving
  `open-game-theory-ontology/` is missing or lacks standard Git tracking.
- Task 1.2: Initialize `open-game-theory-ontology` as a standalone tracked
  subrepository with schema provenance, license, README, and fixture layout.
- Task 1.3: Add failing Turtle and JSON-LD ingestion fixtures for equivalent
  ontology classes, relationships, and annotations.
- Task 1.4: Implement a Rust ontology parser that normalizes Turtle and JSON-LD
  inputs into one internal ontology model.
- Task 1.5: Add deterministic diagnostics for malformed triples, ambiguous
  identifiers, unsupported JSON-LD contexts, and schema version mismatches.

## Phase 2 - Rust component generation

- Task 2.1: Add failing golden-output tests for generated component structs and
  relation metadata.
- Task 2.2: Implement code generation from ontology classes to Rust component
  definitions without hand-editing generated output.
- Task 2.3: Map ontology relationships to graph edge component definitions such
  as `ChildOf(EntityId)` and `TransitionTo(EntityId)`.
- Task 2.4: Add checksum or snapshot evidence so generated structs are
  reproducible across Windows and Linux toolchains.
- Task 2.5: Document the generated-code review boundary and the non-generated
  public API boundary.

## Phase 3 - Kairos Graph-ECS module

- Task 3.1: Add failing compile tests proving graph-relations symbols are not
  exported in default or `--no-default-features` builds.
- Task 3.2: Add an explicit `graph-relations` Cargo feature to the relevant
  package manifests.
- Task 3.3: Implement `ChildOf` and `TransitionTo` as plain data components
  carrying `EntityId` values.
- Task 3.4: Implement traversal helpers over dense component stores using
  `EntityId` lookups, with deterministic ordering, missing-target handling, and
  cycle/depth guards.
- Task 3.5: Add static and unit tests proving graph topology does not use raw
  pointers, self-references, boxed node graphs, `Rc`, or `Arc`.

## Phase 4 - Multi-game framework implementation

- Task 4.1: Add failing tests for ontology-backed components including
  `PayoffMatrix`, `StrategySpace`, `Utility`, players, actions, information
  sets, and terminal nodes.
- Task 4.2: Implement normal-form game execution over flat ECS arrays without
  graph traversal requirements.
- Task 4.3: Implement extensive-form game traversal using `ChildOf` and
  `TransitionTo` components behind the `graph-relations` feature.
- Task 4.4: Add solver systems for normal-form payoff evaluation and
  extensive-form traversal/backward-induction fixtures.
- Task 4.5: Add cross-mode fixtures showing shared ontology components can
  execute both matrix and tree game forms.

## Phase 5 - Documentation, examples, and compatibility

- Task 5.1: Add docs for ontology source provenance, generation flow, feature
  flags, and graph topology constraints.
- Task 5.2: Add minimal normal-form and extensive-form examples with deterministic
  expected outcomes.
- Task 5.3: Run compatibility review for public game-theory structs and solver
  APIs before binding or release claims.
- Task 5.4: Add model-zoo inventory entries only after examples compile and
  conformance fixtures pass.

## Phase 6 - Closeout

- Task 6.1: Run `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and
  all Track 56 test-matrix commands.
- Task 6.2: Run `$conductor-review` and apply accepted in-scope fixes.
- Task 6.3: Commit, push, record commit SHAs and evidence paths in `handoff.md`,
  and keep the track below `Done` until every required gate has real evidence.

## TDD rule

Every implementation task after Phase 0 must add the failing test or validator
first, capture the failure in the task log or handoff, implement the smallest
passing slice, rerun the relevant gate, and commit only a passing repository
state.

## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next
phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update `conductor/tracks.yaml`, `conductor/tracks.md`,
   `conductor/phase-closeout.yaml`, `conductor/status.md`,
   `conductor/implementation-readiness.md`, and `conductor/track-map.md` when
   readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`
   plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in
   `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
8. Advance only after there is no in-scope unstaged or untracked work except
   documented draft satellites.

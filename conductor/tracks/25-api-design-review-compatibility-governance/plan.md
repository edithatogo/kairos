# Track 25 Plan: API Design Review & Compatibility Governance

## Phase 0 - Scope lock

### Task 0.1 - Read the compatibility contracts
- Review the core, FFI, Arrow, conformance, and release contracts already in the repo.
- Record which surfaces are protected and which remain provisional in the docs and track files.
- Open an ADR if public API, ABI, schema, or governance commitments change.
- Treat `conductor/contracts/versioning-compatibility.md` as the source of truth for stable, experimental, and migration-only surfaces.

### Task 0.2 - Lock the owned surface
- Keep the work to `conductor/tracks/25-api-design-review-compatibility-governance/`.
- Update only `conductor/delivery-readiness-checklist.md`, `conductor/quality-gates.md`, and `conductor/contracts/versioning-compatibility.md` outside the track folder.
- Capture any overlap with release or docs as a handoff note rather than a code change.

## Phase 1 - Build the compatibility pack

### Task 1.1 - Define protected surfaces
- Inventory the Rust, C ABI, Arrow schema, and host APIs that are public or semi-public.
- Mark which surfaces are stable, experimental, or migration-only.
- Add one concrete example per surface instead of generic prose.
- Name the live package roots explicitly: `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, `bindings/python`, `bindings/r`, `bindings/julia`, `bindings/typescript`, `bindings/csharp`, and `bindings/go`.

### Task 1.2 - Define the review rules
- State what counts as a breaking change.
- State when an ADR is required.
- State what release stage each class of change can reach.
- State when a migration note is required and when a release hold is mandatory.

## Phase 2 - Wire the gates

### Task 2.1 - Update global readiness docs
- Add compatibility-review rows to `conductor/delivery-readiness-checklist.md`.
- Add compatibility checks to `conductor/quality-gates.md`.

### Task 2.2 - Make the checks machine-readable where possible
- Prefer surface inventories, diff checks, and manifest validation.
- Keep human review for release-stage decisions and exceptions.
- Require the compatibility policy to cite the specific crate or package root affected by each reviewed change.

## Phase 3 - Handoff and release planning

### Task 3.1 - Prepare maintainer notes
- State exactly which changes need ADRs, migration notes, or a release hold.
- List the compatibility artifacts that release planning must inspect.
- Include package-root alignment notes so release planning can tell whether a rename, split, or merge is allowed.

### Task 3.2 - Cross-track communication
- Provide a short handoff for release, docs, and red-team subagents.
- Do not ask implementation workers to write the compatibility policy text.

## Phase 4 - Closeout

### Task 4.1 - Run the docs gates
- Check markdown links.
- Validate Mermaid diagrams.
- Confirm the release checklist references this track's compatibility gates.

### Task 4.2 - Update the risk register
- Mark resolved risks as mitigated.
- Escalate unresolved compatibility risks to release blockers.

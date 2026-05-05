# Track 28 Plan: Red Team & Devil's Advocate Review

## Phase 0 - Scope lock

### Task 0.1 - Read the release contracts
- Review core, FFI, Arrow, conformance, versioning, and release contracts.
- Record which release claims this track can challenge and which it can only escalate.
- Open an ADR if the track needs to change a published compatibility or trust promise.

### Task 0.2 - Lock the owned surface
- Keep the work to `conductor/tracks/28-red-team-devils-advocate-review/` and `reviews/red-team-report.md`.
- Update only `conductor/delivery-readiness-checklist.md` outside the track folder.
- Capture any overlap with security, API governance, or release as a handoff note rather than a code change.

## Phase 1 - Build the adversarial pack

### Task 1.1 - Define the claim-versus-capability ledger
- List the claims made in public docs, release notes, and compatibility pages.
- Match each claim to a concrete artifact or flag it as unsupported.
- Include at least one real counterexample per major release claim.

### Task 1.2 - Define the blocker rubric
- State how a finding becomes a blocker, a warning, or a note.
- State who owns the remediation for each class of finding.
- State when the track should re-run before a release stage.

## Phase 2 - Wire the gates

### Task 2.1 - Update the release checklist
- Add red-team signoff and blocker-closure rows to `conductor/delivery-readiness-checklist.md`.
- Keep the checklist limited to release-facing evidence.

### Task 2.2 - Make the checks machine-readable where possible
- Prefer report freshness, blocker state, and owner presence checks.
- Keep human review for severity and release-hold decisions.

## Phase 3 - Handoff and release planning

### Task 3.1 - Prepare release-manager notes
- State exactly which findings block alpha, beta, RC, or 1.0.
- List the claims that must be downgraded or removed before release.

### Task 3.2 - Cross-track communication
- Provide a short handoff for security, release, docs, and API governance subagents.
- Do not ask other workers to author the red-team findings.

## Phase 4 - Closeout

### Task 4.1 - Run the docs gates
- Check markdown links.
- Validate Mermaid diagrams.
- Confirm the readiness checklist references the blocker path.

### Task 4.2 - Update the risk register
- Mark resolved risks as mitigated.
- Escalate unresolved critical findings to release blockers.

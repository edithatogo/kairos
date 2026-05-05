# Track 20 Plan: OpenSSF, Supply Chain Trust & Institutional Readiness

## Phase 0 - Scope lock

### Task 0.1 - Read the release contracts
- Review core, FFI, Arrow, conformance, versioning, and release contracts.
- Record which release claims this track can approve and which it can only escalate.
- Open an ADR if the track needs to change a published compatibility or trust promise.

### Task 0.2 - Lock the owned surface
- Keep the work to `conductor/tracks/20-openssf-supply-chain-institutional-trust/`.
- Update only `conductor/delivery-readiness-checklist.md` and `conductor/quality-gates.md` outside the track folder.
- Capture any overlap with CI, release, or docs as a handoff note rather than a code change.

## Phase 1 - Build the release-evidence pack

### Task 1.1 - Define the trust checklist
- Name the minimum evidence for alpha, beta, RC, and 1.0.
- Include `SECURITY.md`, `CODEOWNERS`, `.github/workflows/scorecard.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/sbom-attestations.yml`, `.github/workflows/release-attestations.yml`, SBOM, provenance, and release note requirements.
- Mark unsupported tooling as allowed-failure only while the toolchain is unavailable.

### Task 1.2 - Define exception handling
- Document how an exception is recorded, who can approve it, and what stage it blocks.
- Distinguish temporary operational exceptions from permanent policy waivers.

## Phase 2 - Wire the gates

### Task 2.1 - Update global readiness docs
- Add the concrete OpenSSF and supply-chain checks to `conductor/delivery-readiness-checklist.md`.
- Add the corresponding commands or gates to `conductor/quality-gates.md`.

### Task 2.2 - Make the checks machine-readable where possible
- Prefer file-existence, workflow-presence, and artifact-generation checks for the exact workflow files above.
- Keep human review only for policy exceptions, release waivers, and any allowed-failure toolchain gap.

## Phase 3 - Handoff and release planning

### Task 3.1 - Prepare release-manager notes
- State exactly which missing items block alpha, beta, RC, or 1.0.
- List the artifact names and workflow files that a release manager should inspect.

### Task 3.2 - Cross-track communication
- Provide a short handoff for CI, release, and red-team subagents.
- Do not ask other workers to author the policy text that this track owns.

## Phase 4 - Closeout

### Task 4.1 - Run the docs gates
- Check markdown links.
- Validate Mermaid diagrams.
- Confirm the readiness checklist references this track's gates.

### Task 4.2 - Update the risk register
- Mark resolved risks as mitigated.
- Escalate unresolved supply-chain risks to release blockers.

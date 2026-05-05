# 15 Packaging, Publishing & Delivery — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`.
- Read `conductor/package-matrix.md`, `conductor/package-catalog.md`, and `conductor/release-engineering.md`.
- Confirm this track is planning-first and non-destructive.
- Create or refresh `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Naming and registry alignment

- Confirm the preferred package names and fallback names for each ecosystem.
- Record any naming conflicts or reservations as release blockers.
- Note which registry or distribution channel is first for each ecosystem.
- Flag any ADR requirement before public API or package metadata changes.

## Phase 2 — Actionable release map

- Map each ecosystem to a first artifact, first validation command, and first registry target.
- Keep Rust and C ABI first.
- Keep Python, TypeScript, and C# as staged preview lanes.
- Keep R and Julia on GitHub/R-universe/dev-registry style distribution until native delivery is stable.
- Keep Go as semantic-tagged module release planning only.

## Phase 3 — Dry-run readiness

- Define the minimal package metadata each ecosystem needs.
- Define the dry-run or pack/inspect command for each ecosystem.
- Define which docs must mention versioning, loading strategy, and platform support.
- Add concrete conformance or smoke-test commands for ecosystems that can validate locally, and record registry-only checks as deferred follow-ups.

## Phase 4 — Cross-track integration

- Check the plan against the contract and conformance tracks.
- Update docs and release notes where a registry assumption changes.
- Ensure no production publish steps are introduced before the gates are satisfied.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record remaining unknowns and registry follow-ups.
- Confirm the release plan is actionable for the next implementation wave.

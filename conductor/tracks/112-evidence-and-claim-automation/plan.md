# Track 112 implementation plan

Keep public claims traceable to precise evidence and prevent placeholders, historical receipts or interface-shape tests from becoming false acceptance.

## Phase 0: Current truth

- [x] 112.0.1: Maintain the typed capability register with completion owners, exact baseline, current boundaries and missing gates. It links exact installed, scientific and native receipts.
- [x] 112.0.2: Generate the human capability table from the manifest; link current README/homepages and release programme.
- [x] 112.0.3: Label synthetic scaling results as aggregation fixtures and preserve the historical measurements. The register also links the current paired conservative-PDES comparison.
- [x] 112.0.review: Accept the 27-entry register, owners, source-specific evidence and explicit limits. [Review](handoff.md#phase-closeout-evidence)

## Phase 1: Executable guards

- [x] 112.1.1: Validate manifest consistency, known capability states, evidence paths/hashes and required track links.
- [x] 112.1.2: Reject placeholder performance promotion and native, empirical and frontier claims without evidence for that capability and claim class.
- [x] 112.1.3: Add negative mutation cases for tampered results, missing native proof, unsupported status promotion and reversed consumer dependency.
- [x] 112.1.4: Run the existing manifest and guard tests in Docs Quality CI.
- [x] 112.1.review: Accept category, profile and digest guards after their negative controls pass. [Review](handoff.md#phase-closeout-evidence)

## Phase 2: Traceability and maintenance

- [x] 112.2.1: Validate source, target, lock, input, environment, command, output and review identity for new qualification receipts. [First native receipt](../110-scientific-method-completion/evidence/native-qualification-20261125.json)
- [x] 112.2.2: Generate a shared capability table for current docs and release review. Retain each earlier receipt's source revision and scope. [Generated claims](../../capabilities.md) · [Release candidate use](../113-beta-and-stable-release/handoff.md#native-core-v1-decision--25-november-2026)
- [x] 112.2.3: Test stale generated output, recovery and category, platform, profile and source mismatches. The check accepted all 27 current entries and 20 focused tests took 1.41 seconds on Apple M1 Max with Node 26.13.1. Adding the native qualification took one receipt, one retained output and one manifest edit before regenerating the table.
- [x] 112.2.review: Accept the typed run receipt and generated-table drift recovery. [Closeout decision](handoff.md#phase-closeout-evidence)

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.


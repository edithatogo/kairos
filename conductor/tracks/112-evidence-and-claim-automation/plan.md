# Track 112 implementation plan

Keep public claims traceable to precise evidence and prevent placeholders, historical receipts or interface-shape tests from becoming false acceptance.

## Phase 0: Current truth

- [ ] 112.0.1: Create a typed capability register with original/new owners, exact baseline, current boundaries and missing gates.
- [ ] 112.0.2: Generate the human capability table from the manifest; link current README/homepages and release programme.
- [ ] 112.0.3: Correct synthetic scaling certification prose and preserve actual measured historical evidence.
- [ ] 112.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 1: Executable guards

- [ ] 112.1.1: Validate manifest consistency, known capability states, evidence paths/hashes and required track links.
- [ ] 112.1.2: Reject placeholder performance promotion and native/empirical/frontier claims without their specific evidence class.
- [ ] 112.1.3: Add negative mutation cases for tampered results, missing native proof, unsupported status promotion and reversed consumer dependency.
- [ ] 112.1.4: Wire focused checks into existing CI with no credentials or duplicate dependency stack.
- [ ] 112.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Traceability and maintenance

- [ ] 112.2.1: Bind future run receipts to exact source/lock/input/environment/command/output identities and an independent reviewer.
- [ ] 112.2.2: Generate release/docs claims from accepted records; historical receipts remain marked historical on source changes.
- [ ] 112.2.3: Extend guard tests to new entry points and drift reports; measure false rejects and maintenance effort before broader automation.
- [ ] 112.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.


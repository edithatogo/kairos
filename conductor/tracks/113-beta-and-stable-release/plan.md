# Track 113 implementation plan

Release a precisely scoped beta, then an independently reproducible and compatible stable core, without implying all experimental modules are complete.

## Phase 0: Candidate scope

- [ ] 113.0.1: Freeze supported APIs, platforms and capabilities; identify exclusions and their owned completion tracks.
- [ ] 113.0.2: Complete every alpha-to-beta gate in release-gates.md, including immediate Rust-only enforcement.
- [ ] 113.0.3: Qualify clean binary/package installation and an installed CareOps consumer against exact artefacts.
- [ ] 113.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 1: Beta and release candidate

- [ ] 113.1.1: Prepare versioned beta artefacts and compatibility/migration notes; publication remains a separate authorised operation.
- [ ] 113.1.2: Gather complete beta workflows, an independent installation/reproduction and a held-out model evaluation.
- [ ] 113.1.3: Close correctness, replay, native-lifetime and high-impact operational defects; prove upgrade/rollback and recovery.
- [ ] 113.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Stable

- [ ] 113.2.1: Freeze stable API/schema/checkpoint/seed compatibility and support policy.
- [ ] 113.2.2: Pass all existing release/security/provenance gates and final installed-artefact/platform consumers at the RC.
- [ ] 113.2.3: Prepare exact tag/package/archive publication packet, obtain the applicable publication authority, publish and verify separately.
- [ ] 113.2.4: Retain experimental capability labels until their individual scientific/native/performance gates close.
- [ ] 113.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.


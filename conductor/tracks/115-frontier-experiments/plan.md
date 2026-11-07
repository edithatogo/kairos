# Track 115 implementation plan

Evaluate specific research advances beyond established ingredients with falsifiable hypotheses, held-out workloads and publicly reproducible evidence.

## Phase 0: Protocol

- [ ] 115.0.1: Review primary comparator literature and identify exact capability/assumption differences.
- [ ] 115.0.2: Pre-register hypotheses/endpoints/resource budgets for execution planning, branching/CRN and executable claim contracts.
- [ ] 115.0.3: Freeze baseline implementations, input datasets/licences, precision contracts and held-out splits.
- [ ] 115.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 1: Implementation and ablations

- [ ] 115.1.1: Implement recorded execution planning behind an experimental API after stable semantics exist.
- [ ] 115.1.2: Implement real checkpoint branching and CRN variants with entity lifecycle/rollback adversarial cases.
- [ ] 115.1.3: Evaluate claim-contract detection against held-out seeded method/backend/data/provenance defects.
- [ ] 115.1.4: Run ablations showing which component contributes improvements and where the approach fails.
- [ ] 115.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Evaluation

- [ ] 115.2.1: Measure planning regret, time/memory/precision tradeoffs, interval coverage and false acceptance/rejection rates.
- [ ] 115.2.2: Compare to strong named baselines with identical useful work and independent reproduction.
- [ ] 115.2.3: Report negative results, model assumptions and transfer limits; no empirical causal claims from simulation alone.
- [ ] 115.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 3: Research delivery

- [ ] 115.3.1: Archive permitted inputs, code, raw results and exact environment/receipts.
- [ ] 115.3.2: Prepare a methods/software paper scoped to demonstrated contributions and update capability claims only after evidence acceptance.
- [ ] 115.3.3: Submission/publication and authorship approval remain separate actions from repository planning.
- [ ] 115.3.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.


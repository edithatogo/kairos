# Track 110 implementation plan

Game-library refinement, 8 November 2026: S01–S04 implementation is coordinated
through receiving [issue 100](https://github.com/edithatogo/new-drug-reimbursement-game/issues/100)
and T17.04–T17.07 using the [extraction map](../../contracts/game-extraction-map.md).
The phase items below retain the scientific obligations and source API/caller
disposition. They do not authorize a second authoritative Kairos solver.
Core release acceptance follows the included capabilities in Track 113, not
completion of every optional method. No phase status changes in this amendment.

Close every stated scientific/advanced method with a correct implementation and independent evidence, preserving explicit unsupported boundaries until qualification.

### Paignia source dispositions — 21 November 2026

- S01 withdrew the public misnamed RK4 entry point in game-theory 0.3.0 and
  retained the experimental projected-Euler trajectory. [Kairos #407](https://github.com/edithatogo/kairos/pull/407)
- S03 applies the [accepted transfer decision](../../contracts/game-extraction-map.md)
  in game-theory 0.4.0. Paignia receives the historical traffic and Prisoner's
  Dilemma oracles. Kairos retains its experimental zero-sum minimax and four
  oracle tests. [Paignia #64](https://github.com/edithatogo/paignia/pull/64)
- The S03 public conformance controls failed at `ae9ae116` because all four
  retired exports still compiled. After removal, 45 all-feature tests and
  seven compile-fail doctests passed on Rust 1.99.1. The isolated S03 removal
  eliminates the product-equilibrium caller before S02 retires the shared
  legacy Lemke–Howson path. The remaining phase work stays in the checklist.
- S02 withdraws the legacy five-symbol mixed-strategy surface in game-theory
  0.5.0 following the S03 transfer. Paignia's accepted bounded support solver
  retains matching pennies, rejects the false uniform witness under strict
  dominance, and evaluates deviation gains for admitted candidates.
  [Paignia mixed-Nash controls](https://github.com/edithatogo/paignia/blob/f363954fe17e2648942d1ac944280b485f865757/docs/validation/mixed-nash.md)
- The five new S02 conformance controls failed at `6a69262` because the old
  imports compiled. After removal, 44 all-feature tests and 12 compile-fail
  doctests passed on Rust 1.99.1. Paignia ran 39 mixed-Nash tests on Rust 1.75.0.
- S04 withdrew the legacy public Kuhn solver and retained its test-only
  source control. Paignia records the independent Leduc result comparison.
  [Kairos #406](https://github.com/edithatogo/kairos/pull/406) ·
  [Paignia #63](https://github.com/edithatogo/paignia/pull/63)

## Phase 0: Contract and defect inventory

- [ ] 110.0.1: Bind completion IDs S01–S17 to exact APIs/callers, original owner, numerical/runtime contract and version implications.
- [ ] 110.0.2: Add distinguishing negative controls for Euler called RK4, non-equilibrium Nash output, nonstandard Sobol, identity inference and synthetic RL rewards.
- [ ] 110.0.3: Freeze oracle versions, licences, input hashes and supported parameter/shape ranges before implementation.
- [ ] 110.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 1: Numerical and statistical methods

- [ ] 110.1.1: Complete S01 RK4/continuous contract and S02/S03 actual equilibrium solvers; qualify residuals, degeneracy and convergence.
- [ ] 110.1.2: Complete S04 generic CFR/CFR+ graph support with Kuhn and Leduc references.
- [ ] 110.1.3: Complete S05 corrected bounded Sobol/QMC and sensitivity, S06 CRN/UQ, S07 SBI and S08 competing-risk/hazard validation.
- [ ] 110.1.4: Version changed trajectories and qualify downstream users; do not silently update goldens.
- [ ] 110.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Native advanced capability qualification

- [ ] 110.2.1: Complete S09 genuine inference and S10 real Rust VecEnv/Gymnasium/PettingZoo integration.
- [ ] 110.2.2: Route S11 GPU, S12 distributed recovery, S13 external FMI and S14 native I/O/HPC completion through existing owners.
- [ ] 110.2.3: Complete S15 actual state branching/debugging, S16 real streaming/browser transport and S17 hybrid/resource semantics.
- [ ] 110.2.4: Record analytical correctness, native execution, comparative performance and empirical validity separately for each capability.
- [ ] 110.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 3: Acceptance

- [ ] 110.3.1: Run independent held-out cases and deliberately wrong implementations against every claimed oracle.
- [ ] 110.3.2: Remove unsupported method claims from release scope until their individual acceptance closes.
- [ ] 110.3.3: Record accepted method versions, residual/convergence/coverage results and consumer compatibility; independent scientific review before promotion.
- [ ] 110.3.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.


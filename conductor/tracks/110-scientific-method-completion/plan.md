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
  oracle tests. [Paignia #64](https://github.com/edithatogo/paignia/pull/64) ·
  [Kairos #408](https://github.com/edithatogo/kairos/pull/408)
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
  source control. Paignia records the independent Leduc result comparison,
  including independently trained OpenSpiel checkpoints.
  [Kairos #406](https://github.com/edithatogo/kairos/pull/406) ·
  [Paignia #63](https://github.com/edithatogo/paignia/pull/63) ·
  [Leduc qualification](https://github.com/edithatogo/paignia/blob/0389af46db4557494f465db2c584c80080e9f588/docs/validation/cfr.md#independently-trained-leduc-checkpoints)

## Phase 0: Contract and defect inventory

- [ ] 110.0.1: Bind completion IDs S01–S17 to exact APIs/callers, original owner, numerical/runtime contract and version implications.
  - The [register](../../scientific-completion.md) and [workpack](../../execution/README.md) cover every obligation. The full caller and version map for wider native profiles remains open.
- [x] 110.0.2: Retain distinguishing controls for the retired RK4/Nash APIs, nonstandard Sobol, identity inference and host-owned RL rewards. [Paignia transfer controls](../../contracts/game-extraction-map.md) · [Numerical, inference and queue controls](../../scientific-completion.md#qualified-s05s10-scopes)
- [ ] 110.0.3: Freeze oracle versions, licences, input hashes and supported parameter/shape ranges before implementation.
  - The linked executed scopes record the available fixture identities and supported ranges. Establish equivalent input provenance before extending any wider profile.
- [ ] 110.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 1: Numerical and statistical methods

- [x] 110.1.1: Retire Kairos S01–S03 and record bounded Paignia implementation, convergence and residual evidence. [API and caller dispositions](../../contracts/game-extraction-map.md)
- [x] 110.1.2: Retire the Kairos S04 API and record Paignia CFR/CFR+ qualification on Kuhn and Leduc. [Receiving evidence](../../scientific-completion.md#s01s04-paignia-transfer)
- [x] 110.1.3: Complete the bounded S05 Sobol and sensitivity, S06 CRN, S07 ABC-SMC and S08 competing-risk and hazard contracts. [References and criteria](../../scientific-completion.md#qualified-s05s10-scopes)
- [ ] 110.1.4: Version changed trajectories and qualify downstream users; do not silently update goldens.
  - [ADR 0014](../../../docs/adr/0014-versioned-scientific-methods.md) records the provider migration. Each consumer version and replay case needs its own acceptance.
- [ ] 110.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Native advanced capability qualification

- [x] 110.2.1: Record S09 fixed-shape float32 ONNX CPU and affine inference, and S10 Rust-owned queue with scalar/batch and installed host checks. [Scope and receipts](../../scientific-completion.md#qualified-s05s10-scopes)
- [x] 110.2.2: Route S11–S14 through the existing GPU, distributed, FMI and storage owners and record their bounded native results. [Hardware and method evidence](../../scientific-completion.md#s11-apple-metal-and-cuda-model-qualification)
- [ ] 110.2.3: Complete S15 actual state branching/debugging, S16 real streaming/browser transport and S17 hybrid/resource semantics.
  - S15 covers [checkpoint-backed replay, branch isolation and retained Arrow IPC export](../../scientific-completion.md#s15-native-debugger-qualification). Browser integration and additional production workloads remain open.
  - [x] S15 staged DES allocator replay verified rewind within a same-tick pair, replay of the edited fork and isolation of the parent queue and leases. [Execution and review](handoff.md#s15-staged-allocator-replay--26-november-2026)
  - S16 covers [two OS process TCP and Tonic IPC](../../scientific-completion.md#s16-transport-and-telemetry-qualification). Browser and broker transports remain open.
  - S17 covers [DES and ABM boundary and movement cases](../../scientific-completion.md#s17-resource-and-boundary-qualification). General hybrid synchronization and empirical model validation remain open.
  - [x] S17 bounded allocator dispatch verified an inclusive limit, a same-tick budget, and movement arrival with later allocator work still scheduled. [Execution and review](handoff.md#s17-bounded-allocator-dispatch--26-november-2026)
  - [x] S17 coordinated allocator and model dispatch verified event order across DES and ABM, same-tick lease release, and rejection of backdated events. [Execution and review](handoff.md#s17-coordinated-allocator-and-model-dispatch--26-november-2026)
  - [x] S17 native behavior dispatch verified that the scheduler preview leaves the selected ABM event queued for its own runner, and the model releases the lease at the emitted update tick. [Execution and review](handoff.md#s17-native-behavior-dispatch--26-november-2026)
- [x] 110.2.4: Record the distinct evidence limits for analytical correctness, native execution, comparative performance and empirical validity. [Current dispositions](../../scientific-completion.md#limits-on-performance-and-empirical-claims)
- [ ] 110.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 3: Acceptance

- [ ] 110.3.1: Run independent held-out cases and deliberately wrong implementations against every claimed oracle.
- [ ] 110.3.2: Remove unsupported method claims from release scope until their individual acceptance closes.
- [ ] 110.3.3: Record accepted method versions, residual/convergence/coverage results and consumer compatibility; independent scientific review before promotion.
- [ ] 110.3.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.


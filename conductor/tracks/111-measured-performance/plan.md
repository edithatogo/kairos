# Track 111 implementation plan

Remove demonstrated overhead, establish a measured serial/parallel/GPU crossover and publish reproducible matched-workload results.

## Phase 0: Reproducible baseline

- [x] 111.0.1: Reproduce historical small-workload PDES measurements and freeze a common serial/parallel input tape. [Evidence](handoff.md#matched-ring-and-worker-reuse).
- [x] 111.0.2: Profile thread startup/join, synchronization, allocation, queue, snapshots, serialization and transfer cost. [Native profiles and allocator evidence](evidence/profiling/README.md).
- [x] 111.0.3: Separate initialization, steady state and end-to-end timings; retain raw samples, hardware, inputs and exact source. [Matched PHOLD runs](evidence/phold/README.md) retain the first run and ten pairs with setup, dispatch and total times.
- [x] 111.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision. [Independent baseline review](review-report.md).

## Phase 1: Implement measured optimizations

- [x] 111.1.1: Benchmark persistent Rayon or fixed Rust workers; eliminate per-round OS-thread creation where accepted. The measured opt-in executor reuses a pool across rounds and horizons.
- [ ] 111.1.2: Batch eligible events within safe logical-time bounds and reuse buffers without changing ordering or cancellation.
  The one-pass frontier fold is accepted locally. The wider drain candidate failed the same-LP ordering oracle. [Counterexample and decision](evidence/profiling/README.md#event-batch-boundary).
- [x] 111.1.3: Profile tight SoA versus aligned storage and queue alternatives; retain simpler layouts/heap when no net win. The tight-column and compaction profiles support keeping both existing layouts and the compacting heap. [Storage profile](evidence/profiling/README.md#storage-and-checkpoint-boundaries).
- [x] 111.1.4: Keep independent-world ensembles distinct from parallel execution inside one model. The one-worker and eight-worker ensemble results match. [Review checks](review-report.md#local-validation).
- [ ] 111.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Crossover and advanced backends

- [x] 111.2.1: Run P1–P5 matrix from performance-programme.md over event cost, message density, lookahead, state size and workers. [Matched workload panel](handoff.md#matched-workload-panel).
- [x] 111.2.2: Qualify actual GPU model work including setup/transfer and distributed recovery/communication overhead. [Bounded device and transport timings](evidence/backends/README.md).
- [ ] 111.2.3: Implement a recorded pre-run executor choice only after the crossover rule is independently validated.
- [ ] 111.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

The PHOLD runs vary event cost, message density, lookahead and LP count.
The held-out panel adds a FIFO comparison with SimPy and a standalone
spatial consumer with delayed cross-region events. Device and transport
results describe the named configurations. Track 55 retains wider scale
and integrated cross-host restart qualification.

## Phase 3: Acceptance

- [x] 111.3.1: Pass semantic/replay checks before interpreting speed; publish all measured regimes including slowdowns. [Parity and measured slowdowns](handoff.md#matched-workload-panel).
- [ ] 111.3.2: Apply predeclared improvement/regression thresholds and dedicated-runner confidence intervals.
  Both shared-host frontier captures met the local adoption rule. The dedicated environment is unavailable. [Handoff](handoff.md#remaining-acceptance).
- [x] 111.3.3: Run held-out workloads and external matched comparators; no universal superiority claims. [Independent SimPy comparison](evidence/queueing/README.md) and [held-out spatial/DES consumer](evidence/spatial/README.md).
- [ ] 111.3.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.


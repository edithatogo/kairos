# Track 111 implementation plan

Remove demonstrated overhead, establish a measured serial/parallel/GPU crossover and publish reproducible matched-workload results.

## Phase 0: Reproducible baseline

- [ ] 111.0.1: Reproduce historical small-workload PDES measurements and freeze a common serial/parallel input tape.
- [ ] 111.0.2: Profile thread startup/join, synchronization, allocation, queue, snapshots, serialization and transfer cost.
- [ ] 111.0.3: Separate initialization, steady state and end-to-end timings; retain raw samples, hardware, inputs and exact source.
- [ ] 111.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 1: Implement measured optimizations

- [ ] 111.1.1: Benchmark persistent Rayon or fixed Rust workers; eliminate per-round OS-thread creation where accepted.
- [ ] 111.1.2: Batch eligible events within safe logical-time bounds and reuse buffers without changing ordering or cancellation.
- [ ] 111.1.3: Profile tight SoA versus aligned storage and queue alternatives; retain simpler layouts/heap when no net win.
- [ ] 111.1.4: Keep independent-world ensembles distinct from parallel execution inside one model.
- [ ] 111.1.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 2: Crossover and advanced backends

- [ ] 111.2.1: Run P1–P5 matrix from performance-programme.md over event cost, message density, lookahead, state size and workers.
- [ ] 111.2.2: Qualify actual GPU model work including setup/transfer and distributed recovery/communication overhead.
- [ ] 111.2.3: Implement a recorded pre-run executor choice only after the crossover rule is independently validated.
- [ ] 111.2.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase 3: Acceptance

- [ ] 111.3.1: Pass semantic/replay checks before interpreting speed; publish all measured regimes including slowdowns.
- [ ] 111.3.2: Apply predeclared improvement/regression thresholds and dedicated-runner confidence intervals.
- [ ] 111.3.3: Run held-out workloads and external matched comparators; no universal superiority claims.
- [ ] 111.3.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision.

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.


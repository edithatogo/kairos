# Track 111 implementation plan

Remove demonstrated overhead, establish a measured serial/parallel/GPU crossover and publish reproducible matched-workload results.

## Phase 0: Reproducible baseline

- [x] 111.0.1: Reproduce historical small-workload PDES measurements and freeze a common serial/parallel input tape. [Evidence](handoff.md#matched-ring-and-worker-reuse).
- [x] 111.0.2: Profile thread startup/join, synchronization, allocation, queue, snapshots, serialization and transfer cost. [Native profiles and allocator evidence](evidence/profiling/README.md).
- [x] 111.0.3: Separate initialization, steady state and end-to-end timings; retain raw samples, hardware, inputs and exact source. [Matched PHOLD runs](evidence/phold/README.md) retain the first run and ten pairs with setup, dispatch and total times.
- [x] 111.0.review: Independently review this phase, apply accepted fixes, execute the named oracles and record the next-phase decision. [Independent baseline review](review-report.md).

## Phase 1: Implement measured optimizations

- [x] 111.1.1: Benchmark persistent Rayon or fixed Rust workers; eliminate per-round OS-thread creation where accepted. The measured opt-in executor reuses a pool across rounds and horizons.
- [x] 111.1.2: Batch consecutive logical rounds in one persistent-pool dispatch. Recompute eligibility and commit messages in each round. Reuse the frontier map and staged outbound buffer within the run. Keep the one-event-per-LP rule verified by the [batching counterexample](evidence/profiling/README.md#event-batch-boundary).
- [x] 111.1.3: Profile tight SoA versus aligned storage and queue alternatives; retain simpler layouts/heap when no net win. The tight-column and compaction profiles support keeping both existing layouts and the compacting heap. [Storage profile](evidence/profiling/README.md#storage-and-checkpoint-boundaries).
- [x] 111.1.4: Keep independent-world ensembles distinct from parallel execution inside one model. The one-worker and eight-worker ensemble results match. [Review checks](review-report.md#local-validation).
- [x] 111.1.review: Verify round ordering and add the budget-error and worker-resume regression. [Independent review](review-report.md#phases-13).

## Phase 2: Crossover and advanced backends

- [x] 111.2.1: Run P1–P5 matrix from performance-programme.md over event cost, message density, lookahead, state size and workers. [Matched workload panel](handoff.md#matched-workload-panel).
- [x] 111.2.2: Qualify actual GPU model work including setup/transfer and distributed recovery/communication overhead. [Bounded device and transport timings](evidence/backends/README.md).
- [x] 111.2.3: Validate the ten recorded choices on an unseen tape and expose explicit serial and pooled benchmark selection. [Executor evidence](evidence/dedicated/README.md#explicit-executor-selection).
- [x] 111.2.review: Check the raw crossover ratios and include the 32-LP profile in both selected modes. [Independent review](review-report.md#phases-13).

The PHOLD runs vary event cost, message density, lookahead and LP count.
The held-out panel adds a FIFO comparison with SimPy and a standalone
spatial consumer with delayed cross-region events. Device and transport
results describe the named configurations. Track 55 retains wider scale
and integrated cross-host restart qualification.

## Phase 3: Acceptance

- [x] 111.3.1: Pass semantic/replay checks before interpreting speed; publish all measured regimes including slowdowns. [Parity and measured slowdowns](handoff.md#matched-workload-panel).
- [x] 111.3.2: Apply predeclared improvement and regression thresholds on four exclusive CPUs of the physical Linux runner. Both tapes satisfy the [baseline and incremental timing gates](evidence/dedicated/README.md#matched-results).
- [x] 111.3.3: Run held-out workloads and external matched comparators; no universal superiority claims. [Independent SimPy comparison](evidence/queueing/README.md) and [held-out spatial/DES consumer](evidence/spatial/README.md).
- [x] 111.3.review: Recompute both dedicated captures, confirm exact state and counters, and bind the replay results to their runtime and oracle identities. [Independent review](review-report.md#phases-13).

## Phase closeout gate

Before completing a phase, run $conductor-review with an independent reviewer and Auto-apply accepted review fixes within the owned slice. Synchronize conductor/tracks.yaml, conductor/tracks.md, conductor/phase-closeout.yaml and conductor/status.md. Run scripts/validate_conductor_phase_gates.ps1 and the specific test matrix. Commit and push the cleaned slice; then run scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree. Record exact source, command, input/toolchain/artefact identity, exit status and the next-phase decision. No phase advances from planning or metadata checks alone.


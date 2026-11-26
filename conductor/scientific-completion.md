# Scientific and advanced-capability completion register

The obligation inventory dates to 7 November 2026 at `d56863505d938feb069af88a737eb7668113d14b`. The qualification results below were reconciled on 25 November 2026. [Track 110](tracks/110-scientific-method-completion/index.md) follows the original capability owners. Each accepted scope follows its own evidence.

Every advertised method must end with an accurate supported implementation plus evidence, or an explicit unsupported/withdrawn API and documented replacement decision. A placeholder, fixture-shape test or renamed benchmark cannot close a scientific acceptance task. Exclusion from beta does not remove the completion obligation.

| ID | Method/capability and existing owner | Original completion obligation and independent oracle |
|---|---|---|
| S01 | Continuous replicator/RK4 (65,102) | Preserve a named legacy Euler baseline; implement the promised four-stage method or explicitly version/rename a supported Euler API. Define time-varying fitness/model callbacks. Test analytic logistic replicator, step refinement, event discontinuities, positivity and conservation; clipping alone is not accuracy. |
| S02 | Mixed Nash/support enumeration/Lemke–Howson (60,65) | Implement actual supported algorithms and reject unsupported dimensions. Test dominance, matching pennies, coordination, degeneracy, multiple equilibria and rectangular games against Gambit/OpenSpiel; report best-response residual/exploitability. Include the source-derived strictly dominant-action counterexample. |
| S03 | Correlated equilibrium (65,85) | Form actual incentive constraints and solve the intended LP/conic program using qualified existing Clarabel where appropriate. Verify feasibility, incentive residuals and objective against independent solutions; a product of a questionable Nash output is not a general CE solver. |
| S04 | CFR/CFR+ (61,65) | Operate on general supported information-set graphs, chance transitions and legal actions. Separate Kuhn-specific helpers. Compare Kuhn and Leduc policy value/exploitability against OpenSpiel and test known game values, iteration scaling and imperfect-recall rejection. |
| S05 | Sobol/QMC and sensitivity (66,92) | Fix or retire the nonstandard legacy multidimensional generator, checked index bounds and 2^32 overflow. Preserve old trajectories under explicit version labels. Qualify direction numbers, dimension/precision/licence and SciPy/Joe–Kuo vectors. Test Ishigami/Sobol-G analytical sensitivities; sequence correctness does not validate a sensitivity estimator. Replace fixed estimate-times-0.9/1.1 fields with a justified interval procedure and repeated-sample coverage tests; those multipliers are not statistical confidence bounds. |
| S06 | CRN, Monte Carlo and uncertainty (21,66,71,72) | Confirm coupling survives scenario branching by entity/purpose identity. Test distributional calibration, confidence-interval coverage and estimator bias across repeated independent seeds. Record RNG and distribution versions; do not silently replace goldens. |
| S07 | SBI/calibration (66) | Specify likelihood/summary-statistic/estimator family and identifiable parameter space. Run simulation-based calibration/rank checks, posterior-predictive checks and held-out scenarios against analytical or independent references. The current ABC-SMC-named routine evaluates a fixed particle batch with distance weights; implement and verify sequential tolerance schedules, resampling, perturbation kernels, importance weights, ESS/degeneracy handling and posterior convergence before calling it ABC-SMC. No 'SBI' label for a parameter grid alone. |
| S08 | Competing risks/non-Markovian hazards (69) | Verify exponential/Weibull/piecewise hazard inversion, competing cause incidence, censoring, interrupted/restarted clocks and analytical special cases. Compare cumulative incidence and survival to independent references with sampling uncertainty. |
| S09 | ML inference and surrogate (37,89) | Replace copied-input OrtSession execution with an actual qualified Rust ONNX/Tract adapter or explicit unavailable error. Check independently computed outputs, unsupported operators, model provenance, shapes and finite values; evaluate surrogate error and extrapolation. |
| S10 | VecEnv/Gymnasium/PettingZoo (65,109) | Rust owns each world's state, RNG, action effects, rewards and termination. Host adapters only marshal calls. Compare batched vs scalar Rust trajectories; actual checker passes plus native-dispatch proof, seeding/reset/error/buffer-lifetime tests. Remove synthetic rewards/observations as purported execution. |
| S11 | GPU ABM/DES (32,33,52) | Qualify model semantics beyond bounded particle kernels, actual device selection and transfer ownership. Distinguish exact integer replay, tolerance-based float agreement and statistical equivalence. Publish device-specific end-to-end benchmarks. |
| S12 | PDES/Time Warp/distributed recovery (34,35,47,48,49) | Extend accepted bounded runtime to adversarial causal/cancellation/checkpoint cases and real scaling. Test serial parity, stragglers, anti-messages, GVT, crash fences and resource bounds; retain accepted fixed-LP entity-transfer scope. |
| S13 | FMI 2/3 (38,53,91) | Run externally supplied Reference FMUs and independently built OpenModelica cases, including supported events and save/restore. A self-authored mock shared library is a lifecycle test only. Verify named test targets actually exist and handle capability rejection. |
| S14 | Native I/O/NUMA/HPC (50,51,54,55) | Implement actual claimed HDF5/ADIOS2/Arrow backends, engine restart, allocator/affinity semantics and real scheduler/filesystem paths. Custom contract bytes are not HDF5/ADIOS2; job manifests are not completed jobs. Publish measured cross-host/scaling evidence. |
| S15 | Debugging/CoW/branching (40,66) | Operate on real Rust checkpoints and event/state history; prove forward-back-forward equivalence, branch isolation and bounded memory. Measure retained bytes, peak RSS and branch cost; clean/dirty page-count ratios do not establish memory savings. UI playback of fixture data is labelled playback, not engine time travel. |
| S16 | Streaming/WebSocket/telemetry (36,103,104) | Retain typed framed-TCP proof separately; qualify actual RFC6455/browser clients before claiming WebSocket support. Measure ordering, backpressure, dropped/conflicting events, schema evolution and actual engine instrumentation. |
| S17 | DES/ABM/hybrid/resource semantics (03,64,67,68,107) | Analytical queues and adversarial resource allocation, hierarchy lifetime, access-aware routes, boundaries and hybrid time consistency. Model-equivalence tests must execute Rust, not simply validate fixture metadata. |

## Completion protocol

### Ownership after ecosystem separation

S01–S04 move with the reusable game methods to the separate game-library layer
under [ADR 0011](../docs/adr/0011-ecosystem-boundaries.md). Track 110 retains the
obligation until a receiving package, owner, algorithm/result contract and
numerical acceptance are recorded. Keep the existing Kairos API unsupported or
experimental until corrected or explicitly retired. No task closes merely
because source moved to another repository. The remaining IDs stay with their
existing capability owners; S17 additionally consumes the neutral
[movement/resource-usage contract](contracts/movement-resource-usage.md).

The [source extraction map](contracts/game-extraction-map.md) binds the receiving
work to [game-runtime issue 100](https://github.com/edithatogo/new-drug-reimbursement-game/issues/100)
and T17.04 (S01), T17.05 (S02), T17.06 (S03), T17.07 (S04).
Implement each corrected generic method once in that receiving layer. Source-side
Kairos regressions characterize the old API and its replacement/withdrawal; they
are not instructions to build another permanent solver. Preserve the original
S01–S04 oracle requirements and record the accepted receiving artifact plus the
old caller/API disposition before closing a transfer. A library may release a
smaller explicitly supported static capability while other methods remain open.
Neither that release nor a runtime-only Kairos beta closes those methods.

The [bounded workpack](execution/README.md) provides a distinguishing regression
task for every S01–S17 obligation and decomposes immediate fixes and contracts.
An advanced algorithm requiring unresolved design is assigned to a specialist;
it is not disguised as a small local-model implementation task. Design tasks
must produce the next bounded leaves before that implementation is dispatched.

For each ID, record public API and actual callers, precise mathematical/runtime contract, an independent oracle, held-out cases, acceptance thresholds, current evidence class, proposed dependency, output-version implications and owner. Add failing tests that distinguish the real algorithm from the existing approximation before implementation. Preserve negative results. Freeze reference versions/licences/data hashes.

Errors on unsupported inputs must be explicit. Numerical changes require an algorithm/RNG/schema version and consumer qualification. Do not copy implementation logic into the oracle. Production Rust dependencies are distinct from external test-only Python/C++/R references.

[Dataset and oracle catalogue](validation-datasets.md), [performance programme](performance-programme.md) and [release gates](release-gates.md) define the complementary evidence. Algorithmic correctness, software verification, performance and empirical model validity remain separate decisions.

## S01–S04 Paignia transfer

Kairos retired the four legacy public solver APIs. The extraction map records
each removed API and receiving algorithm. Paignia qualifies bounded replicator,
mixed Nash, correlated-equilibrium and CFR/CFR+ cases. Its independently trained
C++ OpenSpiel checkpoints add a Leduc policy value and exploitability comparison
to the Kuhn enumeration oracle. Native CI passed at
`0389af46db4557494f465db2c584c80080e9f588`. Arbitrary games and downstream
adapter acceptance remain with the receiving owners.
[API and caller dispositions](contracts/game-extraction-map.md) ·
[Leduc oracles](https://github.com/edithatogo/paignia/blob/0389af46db4557494f465db2c584c80080e9f588/docs/validation/cfr.md#independently-trained-leduc-checkpoints) ·
[Paignia Native CI](https://github.com/edithatogo/paignia/actions/runs/45431955799)

## Qualified S05–S10 scopes

| ID | Supported scope and verification |
|---|---|
| S05 | Joe–Kuo D(6) matched frozen SciPy 1.18.0 32-bit points across 16 dimensions. The final point and exhausted cursor have separate tests. Jansen first-order and total-effect intervals contained the Ishigami and Sobol-G analytical values in 36–40 of 40 trials per index. [Design and oracle](execution/scientific/s05-completion.md) |
| S06 | The replicate, entity and purpose streams preserve paired draws through a scheduler branch. The observed paired and independent Bernoulli variances were 0.20865 and 0.45514. The paired interval included the true effect in 57 of 60 trials. [Design and oracle](execution/scientific/s06-completion.md) |
| S07 | Sequential tolerance, Gaussian mixture weights and ESS are exercised against the exact beta posterior for two Bernoulli observations. Weighted posterior CDF rank RMSE was 0.02788 across 64 studies. [Design and oracle](execution/scientific/s07-completion.md) |
| S08 | Conditional exponential and Weibull survival and incidence passed analytical bounds at three right-censor horizons. Piecewise inversion, frozen active age, checkpoint replay and cause/censor chronology have exact oracles. The standalone cohort consumer also compares nonproportional piecewise cause incidence and survival at two horizons. [Design and oracle](execution/scientific/s08-completion.md) · [Cohort test](../tests/fixtures/cohort-consumer/src/tests.rs) |
| S09 | The optional ONNX CPU session executes fixed-shape float32 models through the Rust tick hook. Its installed fixture checks a 1,000-tick trajectory against analytical decay. The Rust affine surrogate's maximum absolute error was 0.016443 on 39 interior points. It rejects extrapolation. [Design and oracles](execution/scientific/s09-completion.md) |
| S10 | The native shared queue matched scalar and batched trajectories across 1,024 worlds. Both host checkers execute the queue through the opt-in `queue-env-v1` CLI. Installed Python 3.15.2 tests cover seed rejection, reset, termination, pipe lifetimes and malformed replies. [Design and oracle](execution/scientific/s10-completion.md) |

The S05–S08 core cases and S10 native queue were qualified at
`7ff7eb6a9ec2cb8fa63a7e1a13f8355fc4aed638`. S09's opt-in ONNX qualification
ran at `273bf44956dfadd31336a9f9f645eba6b39779f9`. These results cover
numerical correctness and model execution inside the stated contracts.
[Versioning and migration](../docs/adr/0014-versioned-scientific-methods.md) ·
[Scientific CI](https://github.com/edithatogo/kairos/actions/runs/45026894936) ·
[Installed queue checks](https://github.com/edithatogo/kairos/actions/runs/45026894947) ·
[ONNX CPU CI](https://github.com/edithatogo/kairos/actions/runs/45260627526)

## S11 Apple Metal and CUDA model qualification

An Apple M1 Max ran the resident particle kernel across two workgroups.
Positions agreed with an independent scalar reference within `2e-6` and two
same-device runs produced bitwise identical positions. Empty replacement,
concurrent readback and native validation recovery passed. CPU was faster
in all 60 paired timings for this particle model. The later mixed model
exercised resident additive DES values, signed overflow, snapshots and
device failure recovery. CPU finished first in all 36 matched pairs.
[Metal results](execution/scientific/s11-completion.md) · [Particle timings](execution/scientific/s11-metal-timings.tsv).

The hardware-only integer DES test passed again on Apple M1 Max at
`c5959a9ce92dbbc8991ce4f54480b281099a8e34`.
[Native test receipt](tracks/110-scientific-method-completion/evidence/native-qualification-20261125.json)

The Windows RTX 4060 ran the hardware-only CUDA particle and additive DES
model. Nsight recorded four particle kernels, five DES kernels and both
readbacks after the last kernel. At 65,536 particles and entities with
eight 65,536-event batches, the CPU and CUDA medians were 4.90 ms and 2.26 ms.
Both smaller cases favored the CPU. [CUDA results](execution/scientific/s11-cuda-completion.md).

## S12 distributed protocol qualification

Fourteen conservative protocol cases passed for causal ordering, lookahead,
typed failure and concurrency on macOS ARM64. The cross-environment scope follows
the earlier recorded MPI two/four-rank cases and bounded Mac–Colab gRPC
rollback and crash recovery at `5026d926af1a937a7e58b9621b54c1ae3a3bfe3d`.
Cross-host MPI, cluster scheduling and measured wider scaling remain open.
[Protocol receipt](tracks/110-scientific-method-completion/evidence/native-qualification-20261125.json) ·
[Native transport qualification](tracks/49-distributed-mpi-grpc-state-synchronization/native-runtime-qualification-20261103.md)

## S13 native co-simulation qualification

The FMI 2 and FMI 3 Reference FMU tests execute a native impact and verify
analytical height, velocity and exact checkpoint replay. The Linux
OpenModelica case uses an independently compiled model with a time event.
[Inputs and qualification commands](execution/scientific/s13-completion.md)

## S14 native Arrow and checkpoint qualification

The HDF5 and ADIOS2 adapters write typed native checkpoints and restore the
complete logical sequence. `h5dump` and `bpls` read the timestamp limbs and
signed priority. Self-contained HDF5 and BP5 service queue restarts matched
the uninterrupted run's 228 remaining dispatches and final counters. PyArrow
independently read the typed IPC event, snapshot and checkpoint metadata
streams. Additional restart scenarios, NUMA and live filesystem evidence
remain open. [Evidence](execution/scientific/s14-completion.md).

## S15 native debugger qualification

The Rust replay fixture restored same-tick state and the exact dispatch/RNG
suffix after cancellation. Forks retained independent queue, model and RNG
state. A caught model unwind required restoring the last recorded boundary.
A release build of the CLI replayed one million events, restored position
zero and dispatched them again with identical final state and scheduler output.
The process peaked at 80.8 MiB RSS.

On macOS ARM64, both retention configurations kept the last 1,024 events
of a 65,536-event model with a 64 KiB state and eight pending events during
steady execution. The final state checksum agreed across runs.

| Configuration | Live heap after subtracting baseline | Peak RSS | Median restore | Median fork |
| --- | ---: | ---: | ---: | ---: |
| Full copy at every event | 82.63 MiB | 87.63 MiB | 0.71 µs | 3.21 µs |
| 64-event checkpoints with copy-on-write model pages | 1.51 MiB | 3.86 MiB | 8.38 µs | 0.54 µs |

The heap values come from live `heap` reports. Peak RSS is the median of three
fresh `time -l` runs. Restore and fork times are medians of those runs' 101-sample
medians. Restoring the 63rd event in an interval exercised the periodic case's
longest replay. Each fork edited one model page and scheduled a new event.
These measurements apply to the linked model and settings.
[Receipt](tracks/110-scientific-method-completion/evidence/s15-native-debugger-macos-arm64.json) ·
[Reproduction example](../crates/kairo-ecs-debug/examples/replay_memory.rs) ·
[Checkpoint contract](../docs/adr/0020-native-debugger-replay.md).

The opt-in Rust CLI exports retained dispatches through the replay cursor in
event-log v2 IPC. PyArrow 26.0.1 independently decoded the stream produced
after four steps and a rewind. It contained the single retained dispatch at
sequence 2 and tick 1. This export covers completed dispatch records.
[Arrow stream](tracks/110-scientific-method-completion/evidence/s15-retained-dispatches.arrow) ·
[Execution and readback receipt](tracks/110-scientific-method-completion/evidence/native-qualification-20261125.json)

On the staged DES allocator workload, each debugger step dispatched one
live allocator action. Rewind restored the state between two same-tick
submissions. The parent and an edited fork each replayed the same dispatch
suffix and allocator checkpoint bytes recorded on their own timelines.
The branch withdrew a queued request, admitted the next request at tick 4,
and kept the parent's queue and leases isolated. Browser integration and
additional production workloads remain open.
[Staged allocator reproduction and review](tracks/110-scientific-method-completion/handoff.md#s15-staged-allocator-replay--26-november-2026)

## S16 transport and telemetry qualification

Eighteen cases passed for framed TCP between two OS processes, bounded
queues and reconnect, and Tonic Arrow batch ordering, duplicates and
corrupt input. The supported routes are framed TCP and the qualified Tonic
profiles. Browser RFC 6455 and live Arrow Flight, Kafka and NATS brokers
remain unavailable. The WebSocket decision awaits a live endpoint and an
accepted application protocol for resynchronization and commands.
[Transport receipt](tracks/110-scientific-method-completion/evidence/native-qualification-20261125.json) ·
[WebSocket decision](tracks/103-tungstenite-transport/handoff.md#downstream-evidence-and-entry-decision)

## S17 resource and boundary qualification

Thirteen native DES and ABM cases passed for tick and tie ordering,
conservation, idempotency, queue backpressure and transfer expiry. Movement
cases cover arrival, replanning, cancellation and staged lease cleanup.
The bounded staged allocator dispatch also passed a movement arrival case
with prior and same-tick waiters. The earlier waiter was admitted when the
lease closed at arrival, and the next tick's request stayed scheduled.
[Bounded dispatch evidence](tracks/110-scientific-method-completion/handoff.md#s17-bounded-allocator-dispatch--26-november-2026)

The coordinated step dispatches one eligible event across the staged DES
allocator and the ABM model scheduler. In the movement fixture, admission
starts the journey at tick 2. At tick 10, allocation runs before the arrival,
and the arrival's lease release runs before the next model event. A due
model event also takes precedence over a later eligible allocation. The
allocator fixture checks backdated work in both queues.
[Coordinated dispatch evidence](tracks/110-scientific-method-completion/handoff.md#s17-coordinated-allocator-and-model-dispatch--26-november-2026)

The scheduler preview extends this ordering to native ABM behavior updates.
`BehaviorSimulation` dispatches the selected event and invokes the agent's
handler once. The model uses the emitted update tick to release the lease.
The later allocator request stays scheduled for its own tick.
[Native behavior evidence](tracks/110-scientific-method-completion/handoff.md#s17-native-behavior-dispatch--26-november-2026)

A three-queue case coordinates staged allocation with a separate native
DES scheduler and ABM runner. Model queue order breaks ties after allocation,
and the preview identifies the owner even when the event IDs match. A
separate case rejects a backdated event against the clock of an idle queue.
[Separate model queue evidence](tracks/110-scientific-method-completion/handoff.md#s17-separate-des-and-abm-queues--26-november-2026)

A coordinated forward export can close a usage window while the allocator
is idle. The movement case reports four occupied porter unit-ticks at the
model observation end, censors the held lease, and preserves its later
arrival and release.
[Coordinated occupancy evidence](tracks/110-scientific-method-completion/handoff.md#s17-occupancy-at-a-model-observation-tick--26-november-2026)

General hybrid synchronization and empirical validation of specific models
still require their own results.
[DES and movement receipt](tracks/110-scientific-method-completion/evidence/native-qualification-20261125.json) ·
[Movement contract](contracts/movement-resource-usage.md)

## Limits on performance and empirical claims

The method and native execution decisions follow the individual scopes above.
Comparative performance evidence applies to S11's matched CPU and device
runs, S12's matched conservative PDES ring and PHOLD profiles, S14's local APFS Arrow IPC writes
and S15's model-specific replay retention experiment. Performance comparisons
remain open for S01–S10, S13 and S16–S17. Empirical model validity remains
unassessed across these method and engineering fixtures.
[PDES timing evidence](archive/111-measured-performance/handoff.md#dedicated-qualification) ·
[APFS timing evidence](execution/scientific/s14-completion.md#native-arrow-ipc)

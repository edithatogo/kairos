# Capability reconciliation and execution plan — 20 October 2026

## Decision and authority

The maintainer requested a completion audit, free local execution options, a CareOps/Kairos abstraction review, and updates to both repositories' plans. This is a planning correction, not runtime dispatch, a release acceptance, or a transfer of ownership. Audit baseline: Kairos `f0d30e28c3299dcdb6ca138d95319ecaaf29a54a`; CareOps main `8ca0e5ecec2a41a86ebce32361adbde08dd191d3`; its Kairos pin `44261eb5cf8275f132f63a3396903d116d701dbe`.

The registry records **67 tracks: 60 Done and 7 In Progress (48–53, 55)**. The earlier 59/66 tally is incorrect. These are lifecycle records, not 60 independently verified full capabilities. Tracks 62–66 have **disputed completion claims** against the audited source revision and must not be used as acceptance evidence for dependent upgrades. Preserve their historical closeout records while Track 29 and each owner reconcile scope, reopen incomplete leaves, and synchronize registry/phase ledgers. This audit does not silently reopen tracks or invalidate narrowly accepted historical slices.

## What is complete, and what remains

| Tracks | Source evidence at audited revision | Planning disposition / acceptance still needed |
| --- | --- | --- |
| 47–48 | Single-host conservative runtime and bounded optimistic runtime delivered; earlier implementation-pending descriptions are stale | 47 remains accepted for its bounded scope; 48 needs distributed rollback/GVT/recovery evidence |
| 49 | Native socket multi-process transport and four-rank launcher smoke exist; Cargo has no rsmpi/mpi-sys or tonic/prost transport dependencies | Implement and exercise actual MPI API and gRPC transports if retaining those claims; socket and mpiexec-launched process evidence alone do not satisfy them |
| 50 | `NumaTopology::discover` derives synthetic topology from KAIRO_NUMA_NODES; affinity application/reversion return UnsupportedHost | Real Linux discovery/affinity first; multiple NUMA-node locality measurement remains separate |
| 51 / 62 | Main's `kairo-ecs-arrow` lacks official Arrow/Parquet dependencies; custom ARROW1/PAR1 encodings and a Zstd-named writer do not establish interoperable compressed files; HDF5/ADIOS2 wrappers are contracts | Reconcile existing real CareOps-pin IO implementation before adding another backend; independent readers, compression, corrupt-input and recovery tests required |
| 52 | `metal.rs` reports CpuFallback and calls CPU worker execution; the MSL kernel is empty; no real wgpu/CUDA dependencies | Real device kernel submission, buffers, telemetry, CPU parity and measured timing required; host discovery is not compute proof |
| 53 | FMI2 raw lifecycle surface exists; FMI3 remains reserved | Real FMU loading, lifecycle/error handling and deterministic 1,000-step reference run required |
| 55 | Local workers and ensembles are useful implementation | Real two-host execution and fault/restart evidence before cluster claims; measured scaling before performance claims |
| 56–61 | Ontology parsing/codegen, game graphs and normal/extensive-form bounded solvers exist; focused package tests pass | Keep bounded completion; repair stale handoff text through owners, do not infer universal standards or arbitrary-game certification |
| 63 | Python `_native.py` optionally loads ctypes for get_version; it does not supply a native default scheduler/PyO3 runtime | Native consumer execution, lifecycle, deterministic trace parity and packaging tests before claiming native SimPy replacement; Wasm has separate acceptance |
| 64 | `ConditionWait::AllOf(usize)/AnyOf(usize)` declarations exist without scheduler wait semantics; basic resource/spatial/SoA pieces exist | Implement deterministic composite waits and atomic allocation/cancellation/rollback before downstream reliance |
| 65 | LemkeHowsonSolver delegates a clamped 2x2 indifference helper; strict-dominance counterexample returns a non-equilibrium; Kuhn CFR implementation exists | Fix normal-form solver with independent best-response/NashConv oracles; retain bounded Kuhn evidence, separately qualify general CFR/Leduc |
| 66 | Parallel ensemble/CoW pieces exist; Sobol direction numbers are ad hoc; ABC-SMC surface scores one tolerance population | Reference Sobol vectors and sensitivity benchmarks, true sequential populations/weights/resampling, replay/worker invariance and real browser study required |

## Evidence and its limits

The audit ran `cargo test --locked -p kairo-ecs-game-ontology -p kairo-ecs-game-theory -p kairo-ecs-arrow -p kairo-ecs-gpu -p kairo-ecs-mpi -p kairo-ecs-grpc --all-features --target-dir /tmp/kairos-capability-audit-target` at the Kairos baseline in the isolated audit checkout `/Users/doughnut/.codex/worktrees/kairos-capability-audit/kairos`: exit 0, Homebrew rustc 1.99.1, aarch64-apple-darwin, LLVM 23.1.3. That temporary checkout subsequently became unavailable. No retained full raw test log is claimed; the command result is a session observation. Package success does not establish the missing external capabilities above.

`mpiexec -n 4 /tmp/kairos-capability-audit-target/debug/deps/multirank_smoke-cac21c060b41a92c --exact multirank_smoke_4ranks --nocapture` exited 0 with Open MPI 5.0.11. This launched four OS processes using the implemented socket path, not an MPI API implementation. Hosted core CI at the baseline passed: [run 39129031454](https://github.com/edithatogo/kairos/actions/runs/39129031454). Other scheduled workflows had failures; this is not an all-CI-green statement.

Reproducible solver oracle: row-major payoffs `A=[2,2,1,1]`, `B=[2,1,2,1]`; both players have strictly dominant first actions. Calling `LemkeHowsonSolver::solve(&A,&B)` returns `[0.5,0.5]` for each player and `nash_conv=1`, whereas the pure first-action equilibrium has `nash_conv=0`. Local output `/tmp/kairos-capability-audit-target/dominance-audit.log` SHA-256 `0074d40c75fefc6720d206732aa650410c6863bfbeeb7853e60c8339304cfa05`; fixed deterministic input, no random seed. The local artifact is not a durable CI receipt; the existing solver owner should retain this case in its regression suite.

## Free execution routes and qualification order

No paid service or new hardware purchase is required for the first implementation and qualification leaves. Electricity/storage and installation work are still real costs. Software availability does not mean it has been installed or a workload has passed.

| Host / route | Verified or reported state | Concrete first work and proof boundary |
| --- | --- | --- |
| Mac | Verified M1 Max, 10 CPU / 24 GPU cores, 32 GiB, macOS 27.0.1; Open MPI and Colima installed, Colima stopped | Native Rust tests, official Arrow/Parquet independent-reader checks; actual Metal via wgpu or reviewed native Metal backend. Capture device identity, submitted kernel, output/readback and CPU parity. No CUDA claim |
| Pleiades NAS | SSH-verified Linux x86_64 Intel i3-N300, 8 cores, about 32 GiB, Docker 29.4.0; one NUMA node, Intel render device, no nvidia-smi | Native Linux/x86 portability, real affinity, Docker FMU and single-node Slurm integration; CPU transport peer after matching runtime/architecture review. No NVIDIA, multi-node NUMA, InfiniBand or parallel-filesystem claim |
| Pleiades CUDA computer | Maintainer reports NVIDIA CUDA GPU; separate machine from NAS. SSH probe to pleiades.local:22 timed out | Read back OS/architecture, GPU model, driver/toolkit compatibility, memory and access first; then real CUDA kernel/buffer execution, deterministic CPU parity, failure cleanup and measured transfer/compute times. Hardware acceptance remains pending |
| Colima/Linux containers on Mac | Installed but stopped | Free isolated Linux toolchain/OpenModelica/Slurm smoke route; preserve native Mac Metal lane. Containers do not create NUMA domains, a cluster or CUDA hardware |

Host identity matters: the local SSH alias `pleiades` currently routes to the **NAS**, despite the separate CUDA computer's name. Use a verified explicit host configuration for each before dispatch. Do not overwrite existing aliases, bypass host-key verification, or infer credentials. No remote installation or workload was performed by this planning audit.

Official route references: [wgpu native backends](https://wgpu.rs/), [Colima](https://github.com/abiosoft/colima), [OpenModelica macOS routes](https://openmodelica.org/download/download-mac/), [Open MPI SSH launch requirements](https://docs.open-mpi.org/en/main/launching-apps/ssh.html), [Arrow Rust](https://arrow.apache.org/rust/arrow/index.html), [Parquet Rust](https://arrow.apache.org/rust/parquet/index.html), [Slurm administration quick start](https://slurm.schedmd.com/quickstart_admin.html). OpenModelica's discontinued macOS binary route is not a basis for recommending an unverified Homebrew cask; use its documented container/source route. Mixed macOS/Linux distributed runs require explicit runtime/wire compatibility and cannot be assumed from launch success.

## Ordered owner handoff and acceptance plan

These unchecked leaves are recommendations incorporated into the plan, not new dispatch or claims on the owners' source files.

- [ ] **R0 — Track 29 + owners 62–66:** reconcile disputed Done claims with this source evidence; choose bounded historical closure versus reopening incomplete acceptance, then synchronize tracks.yaml, tracks.md, phase-closeout.yaml and dependency gates. Preserve historical receipts and state why any status changes. Coordinate website correction with existing PR #264 instead of editing its files here.
- [ ] **R1 — owners 03/04/21/22/25 + CareOps upgrade owner:** build a symbol/fixture/dependency/receipt capability matrix for the CareOps pin and main; identify a preserving integration revision; require native ED, Q/C and physical IO parity plus pin rollback before parent pin advancement.
- [ ] **R2 — owners 49/50/51/53:** real transports, Linux topology/affinity, interoperable IO and FMU loading using Mac/NAS. Each leaf must declare dependencies, exact command/input hashes, host/toolchain, exit status and retained raw output; negative tests must distinguish unsupported host from success.
- [ ] **R3 — owner 52 + CareOps E6:** qualify Mac Metal first; qualify CUDA computer identity/access in parallel when available, then implement real CUDA. Compare exact CPU/reference outputs (or specified numerical tolerance), kernel/device evidence and timings. CPU fallback remains explicitly labelled.
- [ ] **R4 — owners 64/65/66:** behavioral correctness gates for composite allocation/waits, strict-dominance solver regression, standard Sobol vectors and sequential ABC-SMC before performance or UI acceptance. Existing source reservations remain intact.
- [ ] **R5 — owners 48/49/54/55 + CareOps E7/E8:** real two-host run with failure injection, checkpoint/restart and deterministic parity; single-node Slurm proves scheduler integration only. Report speedup/efficiency from measured wall time, not constants or worker count. Larger HPC certification remains deferred until genuine infrastructure is available.
- [ ] **R6 — owners 03/21/22/25:** extract only qualified reusable mechanisms from CareOps as described below, with two distinct consuming scenarios and compatibility/recovery evidence.

## CareOps to Kairos boundary

Prioritize reusable experiment orchestration from `batch_runner`/`api_runner`: deterministic seed partitioning, worker/resume equivalence, progress/cancellation, manifests and recovery. Extend existing Kairos experiment/calibration interfaces rather than introduce a second scheduler or runner. Preserve CareOps public facades while changing the backend only after parity.

Next candidates are generic allocation leases, interruption/preemption, handover/work continuation, eligibility predicates and dispatch hooks from operations. Require atomic acquire-all/release/cancel and stale-completion tests first; CareOps `claim_atomic_multi_resource` is currently unsupported and main's AllOf enum is not an implementation. A second consumer (e.g. a nonclinical logistics fixture, followed by a real independent domain adapter) must demonstrate the proposed interface before a broad extraction.

Keep healthcare concepts in CareOps domain/core layers: clinical triage and capability vocabularies, patient/pathway semantics, maternal-neonatal linkage, clinical policy, site staffing rules and empirical mappings. Pass policies into generic primitives; do not import healthcare enums into Kairos. Existing calibration/Flow functionality on the CareOps pin should be reconciled, not newly extracted a second time. The current arrival generator is count-conditioned synthetic ticks, not a qualified Poisson generator.

## Coordination and review

Work uses isolated branches and advisory path claims. Existing Kairos solver/CLI reservations remain protected even when expired; this patch changes no runtime source or owner plan used as a claim input. CareOps' active roadmap owner retains its registry, roadmap, task DAG and proposed upgrade/domain tracks. Its linked addendum carries the same acceptance requirements for integration without editing those owned paths. No chat messages were sent during the 20 October audit and no source claims were recovered.

Review outcome: documentation scope only; no new dependency/API/ABI/schema change, pin change, release or hardware acceptance. The principal remaining risk is stale downstream language treating historical Done records as full capability proof. R0 and the prominent status notices make that discrepancy explicit until owners synchronize lifecycle records. Full `just ci` is not implied by documentation validation; validation results are recorded in the change description.

## Planning validation receipt

Date: 2026-10-20. Working directory: `/Volumes/PortableSSD/codex-worktrees/kairos-capability-plans-20261020`. Base commit: `f0d30e28c3299dcdb6ca138d95319ecaaf29a54a`. Tools: Python 3.15.0, PowerShell 7.6.7, v26.12.0. Deterministic seed: not applicable (documentation validators). Table hashes cover concatenated stdout and stderr of the actual commands; this table is the retained summary artifact, not a substitute for hosted logs. Source inputs are the planning diff against this base; integration must bind the final commit from the PR.

| Command | Exit | Output SHA-256 |
| --- | --- | --- |
| `git diff --check` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | 0 | `adfb3a1b1335bcc0c11862ff3968bad10d42656554bdd2007bd29739e4d883c2` |
| `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1` | 0 | `222356bfa3e996297fe63926559ab3a14d22d8d568657b6f962fa47cbbe0f164` |
| `pwsh -NoProfile -File scripts/validate_conductor_artifacts.ps1` | 0 | `eddb2a82dc6bfa04ced2016884eeaffe1b60b7f0ae3b0e36a44e3ab1feebc324` |
| `node scripts/validation/validate-free-compute-routes.mjs` | 0 | `3a7dcf2d1a12b30d5ae410558559f3461f6138630409a8f41983a303685a421c` |

Full runtime CI was not rerun for these documentation-only changes. Hosted checks and owner lifecycle reconciliation remain separate.

## Review update — 21 October 2026

The source and hardware findings above describe the explicit 20 October revisions/readbacks, not a fresh audit of remote main. GitHub CLI HTTPS requests, Git HTTPS/SSH fetches and the GitHub connector failed during this review; current remote heads, review threads, merge conflicts and hosted check results could not be verified. Do not mark either PR merge-ready from the local checks alone. No runtime capability or track closure is advanced by these review fixes.

Review fixes remove the contradictory unverified OpenModelica cask instruction and fixed free-cloud GPU quota/model promises from the route catalog, distinguish generic TCP from real gRPC, and require source/input/toolchain/raw-artifact hashes for each new local route. The generic macOS tooling route and observed M1 Max inventory remain distinct; neither proves a working GPU backend.

The maintainer explicitly authorized agent coordination on 21 October. The CareOps roadmap owner received C-R0–C-R7 for integration into its existing upgrade/domain plans and task DAG. Existing Kairos CLI/solver reservations remain protected; no corresponding live chat was identified in the available chat inventory. Their R4 requirements remain pending owner acknowledgement, not silently dispatched or accepted.

Paired changes: [Kairos PR #266](https://github.com/edithatogo/kairos/pull/266) and [CareOps PR #173](https://github.com/edithatogo/careops-sim/pull/173). The CareOps plan integration is tracked separately by its owner; integration evidence must identify the actual commit/task mapping.

Local review verification on 21 October at parent `0cdc9ac140c51faf80ee916ba10a7f11e1c70cbd`: `git diff --check` passed; Conductor phase gates, DAG and artifact validators plus the free-compute route validator all passed with no reported errors. Working directory and validator toolchain are the same as the retained planning receipt above. No new tests were added for this prose/catalog correction. Source inspection reconfirmed the audited capability boundaries; full runtime CI was not rerun.

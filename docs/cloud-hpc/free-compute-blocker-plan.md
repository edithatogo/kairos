# Free Compute Blocker Plan

This plan records which residual HPC blockers can be reduced with free or
near-free execution surfaces, and which blockers still require live HPC
hardware or scheduler evidence.

## Routes

| Route id | Surface | Can reduce | Cannot close |
|---|---|---|---|
| `github-actions-standard-public` | GitHub Actions standard hosted runners for public repositories | CPU/container CI, local scaling placeholder checks, Conductor validators, Rust/Node/Python smoke gates | Slurm, MPI multi-node, Lustre/GPFS throughput, native GPU speedup, production weak/strong scaling |
| `github-actions-macos-metal-smoke` | GitHub Actions macOS runner | Metal-adjacent GPU/WebGPU compile and smoke coverage | CUDA, target-HPC `wgpu`, GPU benchmark threshold, persistent device-memory throughput |
| `huggingface-spaces-cpu-basic` | Hugging Face Spaces free basic CPU | Public CPU demo, Linux CPU smoke, local placeholder JSON generation | Scheduler acceptance, parallel filesystem throughput, native GPU execution, MPI cross-node behavior |
| `huggingface-spaces-gpu-request` | Hugging Face Spaces GPU or ZeroGPU-style route when granted or available | Ad hoc GPU enumeration and dependency smoke | Stable CI, guaranteed capacity, Track 52 benchmark threshold, Track 55 certification |
| `docker-container-on-github-actions` | Docker engine in GitHub-hosted Linux workflow | Track 54 Docker image build and CLI smoke through `.github/workflows/cloud-smoke.yml` | Kubernetes reconciliation, Slurm submission, cloud batch execution, checkpoint resume under preemption |

## Current application

- Track 54: GitHub Actions can reduce the Docker image build and CLI smoke
  blocker when `cloud-smoke.yml` passes. It does not close Kubernetes, Slurm,
  AWS Batch, GCP Batch, or Azure Batch runtime acceptance.
- Track 52: `gpu-free-smoke.yml` can reduce GPU-adjacent build and smoke risk.
  It does not replace a completed `live-hpc` GPU hardware manifest.
- Track 55: GitHub Actions and Hugging Face CPU Spaces can run local scaling
  placeholder commands. They cannot close weak/strong scaling certification.
- Track 51: free CPU/container routes can validate Arrow/checkpoint contracts.
  They cannot close Lustre/GPFS or MPI-I/O throughput proof.

## Evidence rules

Every free-route evidence record must include a run URL or Space URL, commit
SHA, command log, runner or hardware tier, and raw output path where applicable.
If the output is used in an HPC evidence manifest, it remains `scaffold` unless
the route actually provides the live hardware/scheduler/filesystem required by
the target track.

## Source notes

- GitHub Actions billing documentation states standard GitHub-hosted runner use
  is free for public repositories.
- GitHub-hosted runner documentation lists the managed runner families used by
  the workflows.
- Hugging Face Spaces documentation describes a free basic CPU environment.
- Hugging Face GPU Spaces documentation describes upgraded GPU hardware and
  free-upgrade requests, but this is availability/request based and not stable
  certification capacity.

The machine-readable route matrix is `conductor/free-compute-routes.json`, and
the validator is `scripts/validation/validate-free-compute-routes.mjs`.

## Zero-Cost macOS & Free-Tier Cloud Execution Playbook (Tracks 48-55)

When developing on an Apple Silicon Mac without a paid multi-node supercomputing allocation, KairoECS separates **repo-complete engineering (`R3` Local/Free-Tier Hardware Proof)** from **external cluster certification (`R4` `live-hpc` Multi-Node Proof)** using these additional validated routes:

1. **`macos-local-apple-silicon-metal-mpi-hwloc` (Local Mac Native Hardware):**
   - **GPU (`Track 52`):** Apple Silicon unified memory supports native **Metal** compute via `wgpu` (`Backends::METAL`). Running `cargo test -p kairo-ecs-gpu --features wgpu-backend` on macOS executes real GPU adapter discovery, WGSL shader compilation, persistent buffer residency, and GPU-to-CPU readback parity at zero cost.
   - **MPI & gRPC (`Track 49`):** Homebrew OpenMPI (`mpiexec --oversubscribe -np 4`) and `protoc` execute real multi-rank OS process collectives, all-reduce GVT, and two-process TCP/gRPC socket synchronization locally.
   - **NUMA & Topology (`Track 50`):** Homebrew `hwloc` (`hwloc-ls`, `lstopo`) and macOS Darwin `sysctl hw.perflevel*` provide real P-core/E-core cache and topology discovery alongside lock-free `EventArena` contention tests.
   - **FMI 2/3 (`Track 53`):** Rust `cdylib` test FMUs compiled to `.dylib` (plus optional `brew install --cask openmodelica`) verify real `libloading` dynamic symbol resolution and 1,000-step co-simulation on macOS.
2. **`macos-colima-docker-slurm-k3s-emulation` (Local Linux Cluster Emulation on Mac):**
   - **Slurm & Kubernetes (`Track 54`):** Using `colima start` (installed via Homebrew) with a containerized Slurm stack (`docker-slurm`) and `k3d`/`kind` (`kubectl`), job submission scripts (`sbatch`, `srun`, Kubernetes `Job` specs) and checkpoint restarts run end-to-end locally.
   - **Parallel I/O (`Track 51`):** `brew install hdf5` + official `arrow`/`parquet` crates verify real binary checkpoint/restart parity locally.
3. **`free-cloud-cuda-sandboxes-colab-kaggle-lightning` (Free NVIDIA CUDA Verification):**
   - **CUDA (`Track 52`):** Free NVIDIA T4/P100 sessions on Kaggle Notebooks (30 hrs/week free), Google Colab, or Lightning.ai Studio install `rustup` and run `cargo test -p kairo-ecs-gpu --features cuda-backend`, producing signed `nvidia-smi` + raw artifact checksums without AWS/GCP GPU spend.

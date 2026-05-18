# Track 39: Cloud / HPC Batch Runners

## Purpose

Production-scale simulation execution. Docker images, Kubernetes operator, AWS Batch / GCP Batch / Azure Batch integration, Slurm job scripts for HPC clusters, containerized experiment orchestration with spot/preemptible instance resilience, S3/GCS/Blob telemetry output.

## Why this track exists

The kairo-ecs-cli from Track 22 gives users a local experiment runner. Production simulation workloads require scale: hundreds of parameter-sweep runs, long-running ensembles on HPC clusters, and burst-compute across cloud providers. This track builds toward a containerized, orchestrated, and cloud-portable execution layer so that users can run KairoECS simulations from a single Docker container through site-managed cloud/HPC batch environments, with explicit validation gates for spot/preemptible interruption handling and cloud-storage telemetry output.

## Primary subagent

`cloud-agent`

## Parallelization model

Depends on Track 22 (experiment runner CLI) and Track 15 (packaging). The CLI binary and package manifests are inputs — this track wraps them, does not modify them. Cloud provider integrations (AWS, GCP, Azure) can be developed in parallel with each other after the Docker image and K8s operator phases. HPC Slurm scripts are independent of cloud batch providers. Spot resilience is cross-cutting and touches every provider.

## Inputs

- `crates/kairo-ecs-cli/` — experiment runner binary from Track 22. Track 22 explicitly handoff-approves the current minimal `run`, `checkpoint`, and `resume` scaffold for Track 39 wrapper compatibility only; production execution, checkpoint state, resume semantics, `collect`, and `analyze` remain Track 22-owned.
- `conductor/tracks/15-packaging-publishing-delivery/` — package manifests and release artifacts for container image build.
- `crates/kairo-ecs-arrow/` — Arrow telemetry output format from Track 04 (read-only contract).
- `conductor/contracts/core-contract.md` — deterministic ordering contract (ensures cloud runs are reproducible).

## Owned paths

- `docker/`
- `k8s/`
- `docs/cloud-hpc/`
- `.github/workflows/cloud-smoke.yml`

## Outputs

- Multi-arch Docker image (`linux/amd64`, `linux/arm64`) packaging `kairo-ecs-cli` with all runtime dependencies.
- Kubernetes CRD (`KairoECSExperiment`) and operator reconciling experiment lifecycle (pending → running → completed/failed).
- Slurm job submission scripts (`hpc/slurm/`) supporting array jobs, GPU partition routing, and checkpoint hooks.
- AWS Batch / GCP Batch / Azure Batch job definitions (`cloud/aws/`, `cloud/gcp/`, `cloud/azure/`) with environment variable templating.
- Spot/preemptible instance checkpoint-and-resume logic: saves experiment state on interruption signal, restores from last checkpoint on next launch.
- Cloud storage telemetry output plugin (`S3`, `GCS`, `Azure Blob`) writing Arrow telemetry with integrity checksums.

## Blocked paths

- `crates/kairo-ecs-cli/` — owned by Track 22. This track consumes the binary, does not modify it except through explicit Track 22 handoff/approval evidence recorded in Track 22 and Track 39 handoff notes.
- `crates/kairo-ecs-arrow/` — owned by Track 04. Arrow schema changes require ADR from Track 04.
- `crates/kairo-ecs-core/` — owned by Track 01. Core scheduler must not be altered for cloud execution.
- `conductor/tracks/15-packaging-publishing-delivery/` — release process owned by Track 15.

## Acceptance criteria

These are release targets, not claims about the current scaffold. Current verified evidence is limited to offline manifest/policy validation unless a live Docker, Kubernetes, Slurm, AWS, GCP, or Azure command is recorded in `handoff.md`.

- Docker image builds successfully for `linux/amd64` and `linux/arm64` and runs `kairo-ecs-cli run` inside the container.
- Kubernetes operator creates a `KairoECSExperiment` CR, launches a pod, and marks the experiment as completed after successful execution.
- Slurm script submits a job array, each job runs a parameter-sweep variant, and the script blocks until all jobs complete.
- Spot instance interruption signal (SIGTERM) triggers a checkpoint write; on next launch, the experiment resumes from the checkpoint with identical final state.
- Telemetry output writes to S3-compatible storage with a SHA-256 checksum file alongside the Arrow output.
- AWS Batch, GCP Batch, and Azure Batch job definitions are validated against provider schemas.

## Current verified scope

- `cloud/validate_cloud_hpc.py` performs offline validation for the `kairo-ecs-cli` command surface, Dockerfile/entrypoint policy, Kubernetes CRD/sample/operator rendering, AWS/GCP/Azure template shape, Slurm signal/checkpoint wiring, checkpoint/spot policy documentation, and telemetry checksum sidecars/provider upload manifests.
- Provider upload is currently represented by local upload manifest generation for `s3://`, `gs://`, and `az://` destinations; live object-store writes require provider-specific implementation and credentials.
- Slurm and cloud provider acceptance require live scheduler/provider validation before readiness is claimed.
- Live runtime blocker tracking for all Docker, Kubernetes, Slurm, and provider acceptance paths is now recorded in `docs/cloud-hpc/runtime-evidence-boundary.md`.

## Release implications

Non-blocking for library release; required for cloud/HPC users. Container images published alongside each release. Cloud batch definitions and Slurm scripts are versioned alongside the release tag. Smoke tests in `.github/workflows/cloud-smoke.yml` run on PRs touching cloud artifacts but do not gate the library release.

## Non-goals

- Modifying the kairo-ecs-cli interface or adding cloud-specific CLI flags (owned by Track 22).
- Altering the Arrow telemetry schema (owned by Track 04).
- Managing cloud infrastructure provisioning (VPCs, subnets, IAM roles) — these are user responsibilities documented in `docs/cloud-hpc/`.
- Orchestrating multi-node distributed simulations (Track 35 owns the MPI/gRPC simulation distribution layer).
- Providing a managed SaaS platform — this track provides self-serve artifacts for users to run in their own cloud/HPC environments.

# Track 39 Plan: Cloud / HPC Batch Runners

## Phase 0 — Contract alignment

### Task 0.1 — Read upstream contracts
- Review `kairo-ecs-cli` subcommand interface (`run`, `collect`, `analyze`) and CLI argument surface from Track 22.
- Review Arrow telemetry output schema and serialization format from Track 04.
- Review container packaging requirements and release artifact structure from Track 15.
- Confirm deterministic ordering contract from `docs/core-contract.md` holds across cloud run environments.

### Task 0.2 — Define owned surface
- Create `docker/` directory with `Dockerfile`, `.dockerignore`, and `docker-bake.hcl` for multi-arch builds.
- Create `k8s/` directory with CRD schema, operator skeleton, and sample experiment manifests.
- Create `docs/cloud-hpc/` with provider-specific setup guides.
- Create `.github/workflows/cloud-smoke.yml` for container build and operator smoke tests.
- Path-guard: this track may read but must not write to `crates/kairo-ecs-cli/`, `crates/kairo-ecs-arrow/`, `crates/kairo-ecs-core/`.

## Phase 1 — Containerize

### Task 1.1 — Multi-arch Docker image
- Write `docker/Dockerfile` that builds `kairo-ecs-cli` from source (or copies pre-built binary from Track 15 release artifacts).
- Configure `docker-bake.hcl` for `linux/amd64` and `linux/arm64` platform targets.
- Include runtime dependencies: minimal base image (distroless or Alpine), CA certificates, timezone data.
- Tag strategy: `kairo-ecs-cli:<version>`, `kairo-ecs-cli:latest`, `kairo-ecs-cli:<version>-<arch>`.

### Task 1.2 — Smoke test the image
- Build image locally, run `docker run kairo-ecs-cli:<tag> run --help` to verify CLI interface.
- Run a minimal experiment inside the container and verify exit code 0.
- Verify telemetry output path writes within the container.

### Task 1.3 — CI integration
- Add `docker build` to `.github/workflows/cloud-smoke.yml`.
- Tag images with git SHA; push to a registry only on release tags.

## Phase 2 — Kubernetes operator

### Task 2.1 — Define the CRD
- Create `k8s/crd/kairoecs-experiment.yaml` defining the `KairoECSExperiment` custom resource.
- CRD spec fields: `scenarioRef` (config map or inline manifest), `parallelism`, `image`, `imagePullPolicy`, `storage.backend` (S3/GCS/Azure), `storage.path`, `resources.limits`, `checkpoint.enabled`, `checkpoint.interval`.
- CRD status fields: `phase`, `completedRuns`, `failedRuns`, `lastCheckpointTime`, `message`.

### Task 2.2 — Implement operator skeleton
- Create `k8s/operator/` with a reconciliation loop (Python or Go).
- On experiment creation: create a ConfigMap from the scenario manifest, spawn pods with the container image, mount scenario and output volumes.
- On pod completion: aggregate exit codes, update CRD status.
- On experiment completion: transition phase to `Completed` or `Failed`.

### Task 2.3 — Operator smoke test
- Deploy CRD and operator to a local kind/minikube cluster.
- Submit a sample `KairoECSExperiment` resource.
- Verify the operator spawns pods, waits for completion, and updates the CRD status correctly.

## Phase 3 — Cloud batch providers

### Task 3.1 — AWS Batch
- Create `cloud/aws/batch-job-definition.yaml` with container image, vCPU/memory requirements, environment variables for telemetry output.
- Create `cloud/aws/batch-array-template.yaml` for parameter sweeps using `AWS_BATCH_JOB_ARRAY_INDEX`.
- Write `cloud/aws/submit-experiment.sh` wrapping `aws batch submit-job`.

### Task 3.2 — GCP Batch
- Create `cloud/gcp/batch-job.json` with `runnable.container` definition.
- Create `cloud/gcp/batch-array.json` for task arrays.
- Write `cloud/gcp/submit-experiment.sh` wrapping `gcloud batch jobs submit`.

### Task 3.3 — Azure Batch
- Create `cloud/azure/batch-job.json` with pool and task definitions.
- Create `cloud/azure/batch-array.json` for multi-instance tasks.
- Write `cloud/azure/submit-experiment.ps1` wrapping `az batch job create`.

### Task 3.4 — Provider validation
- Validate each job definition against its provider's schema (e.g., `aws batch register-job-definition --dry-run`).
- Document required IAM/service-account permissions in `docs/cloud-hpc/`.

## Phase 4 — HPC Slurm

### Task 4.1 — Single-job submission
- Write `hpc/slurm/submit-experiment.sh`: accepts scenario file path, output directory, partition, and node count, generates a Slurm batch script, and calls `sbatch`.

### Task 4.2 — Job array support
- Write `hpc/slurm/submit-sweep.sh`: parameter sweep via Slurm job arrays (`#SBATCH --array=0-N`), each task runs one sweep variant with `SLURM_ARRAY_TASK_ID` as the variant index.

### Task 4.3 — Checkpoint hooks
- Add Slurm `--signal=B:SIGTERM@120` to request preemption notification.
- Trap `SIGTERM` in the job script to trigger `kairo-ecs-cli` checkpoint before the 120-second window expires.
- Write `hpc/slurm/resume.sh` for resubmitting from a checkpoint.

### Task 4.4 — Documentation
- Write `docs/cloud-hpc/slurm.md` covering `sbatch` invocation, array job setup, GPU partition selection, and checkpoint/resume patterns.

## Phase 5 — Spot resilience

### Task 5.1 — Checkpoint signal handler
- Implement interrupt-signal trapping in the Docker entrypoint or a wrapper script (`docker/entrypoint.sh`).
- On `SIGTERM`, invoke `kairo-ecs-cli checkpoint` (or equivalent subcommand) to flush state to the configured output path.
- Write a checkpoint manifest file with the experiment run state, last completed tick, and Arrow output position.

### Task 5.2 — Resume logic
- On next launch, if a checkpoint manifest exists, invoke `kairo-ecs-cli resume --checkpoint <path>`.
- Verify that resumed runs produce identical final state to uninterrupted runs (using the deterministic ordering contract from Track 01).

### Task 5.3 — Local simulation test
- Write a test that launches a container, sends `SIGTERM` mid-run, verifies the checkpoint is written, then relaunches the container and verifies the resume completes with identical output.

## Phase 6 — Telemetry cloud output & closeout

### Task 6.1 — Cloud storage plugin
- Create `docker/telemetry-plugin/cloud-output.py`: reads Arrow telemetry from `kairo-ecs-cli` output, writes to S3/GCS/Azure Blob using the provider SDK.
- Append a SHA-256 checksum file (`<output-key>.sha256`) alongside each telemetry file.
- Support configurable credentials: environment variables, instance metadata, or workload identity.

### Task 6.2 — CI smoke test
- Add S3 telemetry output test to `.github/workflows/cloud-smoke.yml` using a local S3-compatible service (e.g., MinIO or moto).
- Verify that after a successful experiment run, the S3 bucket contains both the Arrow file and a matching checksum.

### Task 6.3 — Cross-track handoff
- Hand off to Track 22 (experiment-agent): confirm container entrypoint does not alter CLI behavior.
- Hand off to Track 15 (packaging-agent): confirm Docker image fits within release artifact pipeline.
- Hand off to Track 35 (distributed-agent): the LP model from PDES may be consumable by cloud runners; note multi-node scheduling is Track 35's scope.
- Notify Track 13 (ci-agent) of new `cloud-smoke.yml` workflow and its resource requirements.

### Task 6.4 — Update risk register
- Mark resolved risks as mitigated.
- Escalate any provider integration that cannot be validated in CI.
- Document spot-resilience limitations for stateful experiments with no checkpoint support.

## Worker 5 evidence — 2026-05-06

Completed with artifact evidence:

- Phase 0/1 owned surface and container scaffold: `docker/Dockerfile`, `docker/.dockerignore`, `docker/docker-bake.hcl`, and `docker/entrypoint.sh`.
- Phase 2 Kubernetes scaffold: `k8s/crd/kairoecs-experiment.yaml`, `k8s/operator/kairoecs_operator.py`, and `k8s/samples/experiment.json`.
- Phase 3 provider scaffolds: `cloud/aws/`, `cloud/gcp/`, and `cloud/azure/` job templates plus submit helpers.
- Phase 4 Slurm scaffold: `hpc/slurm/submit-experiment.sh`, `hpc/slurm/submit-sweep.sh`, `hpc/slurm/resume.sh`, and `docs/cloud-hpc/slurm.md`.
- Phase 5 checkpoint/interrupt scaffold: `docker/entrypoint.sh` writes an atomic checkpoint manifest on `SIGTERM`; Slurm scripts request `--signal=B:SIGTERM@120`.
- Phase 6 telemetry scaffold: `docker/telemetry-plugin/cloud-output.py` writes SHA-256 sidecars and local/provider upload manifests.
- Phase 6 offline validation increment: `cloud/validate_cloud_hpc.py` now renders the sample Kubernetes Job, checks indexed completion/env wiring, copies a local Arrow telemetry fixture with a SHA-256 sidecar, and verifies an `s3://` provider upload manifest without requiring credentials.
- Hardening slice: `cloud/validate_cloud_hpc.py` now also checks Dockerfile/entrypoint invariants, Kubernetes CRD/sample shape, AWS/GCP/Azure template wiring, Slurm syntax and signal/checkpoint wiring, provider documentation disclaimers, and checkpoint/spot policy documentation. Slurm checkpoint defaults are local filesystem paths rather than remote output URIs.

Validation evidence:

- `python cloud\validate_cloud_hpc.py` passed.
- `python k8s\operator\kairoecs_operator.py --experiment k8s\samples\experiment.json` rendered a `batch/v1` Job with indexed completions.
- `python docker\telemetry-plugin\cloud-output.py --input cloud\.validation\telemetry-src\events.arrow --destination cloud\.validation\telemetry-dst` copied telemetry and wrote a matching `.sha256` sidecar.

Not marked complete:

- Provider dry-run commands (`aws`, `gcloud`, `az`) were not run because no cloud account context or credentials are available in this worker scope.
## Phase closeout gate

Before any task or phase in this track is marked complete, and before the next phase begins:

1. Run `$conductor-review` against this track and the current diff.
2. Auto-apply accepted review fixes inside this track's owned paths.
3. Record rejected, cross-track, or blocked-path fixes in `handoff.md`.
4. Update the track registry/status surfaces: `conductor/tracks.yaml` (authoritative machine-readable registry), `conductor/tracks.md` (human index), `conductor/phase-closeout.yaml` (review ledger), `conductor/status.md` (narrative status), and `conductor/implementation-readiness.md` or `conductor/track-map.md` when readiness, ownership, dependency, gate, or wave data changes.
5. Run `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` plus the gates listed in `test-matrix.md`.
6. Commit and push the cleaned slice, then record the commit SHA or blocker in `handoff.md`.
7. Run `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` to verify recorded commits, pushed refs, and cleanup state.
8. Advance the next phase only after there is no in-scope unstaged or untracked work except documented draft satellites.
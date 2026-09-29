# Handoff: Track 39 Cloud / HPC Batch Runners

## Summary

Defined the scaffold and offline validation layer for production-scale KairoECS execution. The current verified scope is the `kairo-ecs-cli` command surface, manifest shape, local rendering, shell syntax, checkpoint/spot policy wiring, and local telemetry checksum behavior. The offline validator passes, but live Docker builds, Kubernetes cluster reconciliation, Slurm scheduler submission, and AWS/GCP/Azure provider API acceptance still require environment-backed validation before any readiness claim.

## Files changed

`conductor/tracks/39-cloud-hpc-batch-runners/spec.md`, `conductor/tracks/39-cloud-hpc-batch-runners/plan.md`, `conductor/tracks/39-cloud-hpc-batch-runners/agent-contract.md`, `conductor/tracks/39-cloud-hpc-batch-runners/risk-register.md`, `conductor/tracks/39-cloud-hpc-batch-runners/test-matrix.md`, `conductor/tracks/39-cloud-hpc-batch-runners/handoff.md`, `docker/Dockerfile`, `docker/docker-bake.hcl`, `docker/entrypoint.sh`, `docker/telemetry-plugin/cloud-output.py`, `k8s/crd/kairoecs-experiment.yaml`, `k8s/operator/`, `cloud/aws/batch-job-definition.yaml`, `cloud/aws/batch-array-template.yaml`, `cloud/aws/submit-experiment.sh`, `cloud/gcp/batch-job.json`, `cloud/gcp/batch-array.json`, `cloud/gcp/submit-experiment.sh`, `cloud/azure/batch-job.json`, `cloud/azure/batch-array.json`, `cloud/azure/submit-experiment.ps1`, `hpc/slurm/submit-experiment.sh`, `hpc/slurm/submit-sweep.sh`, `hpc/slurm/resume.sh`, `docs/cloud-hpc/slurm.md`, `docs/cloud-hpc/aws-batch.md`, `docs/cloud-hpc/gcp-batch.md`, `docs/cloud-hpc/azure-batch.md`, `docs/cloud-hpc/checkpoint-spot-policy.md`

## Contracts consumed

- `crates/kairo-ecs-cli/` — experiment runner CLI from Track 22 (subcommand interface, argument parsing, exit codes).
- `crates/kairo-ecs-arrow/` — Arrow telemetry output format from Track 04 (read-only; schema compatibility is Arrow-agent's scope).
- `conductor/contracts/core-contract.md` — deterministic ordering contract from Track 01 (cloud runs must produce reproducible results).
- Track 15 packaging manifests — release artifact structure for container image consumption.

## Release gates affected

This track is explicitly non-blocking for library release. Container images are published alongside each release as supplementary artifacts. The `cloud-smoke.yml` CI workflow gates PRs touching cloud artifacts but does not gate the library release. Cloud batch definitions and Slurm scripts are versioned alongside the release tag for traceability.

## Concrete artifacts

- **Docker**: `docker build -t kairo-ecs-cli:latest . && docker run kairo-ecs-cli:latest run --help`
- **K8s operator**: `python k8s\operator\kairoecs_operator.py --experiment k8s\samples\experiment.json` for offline render validation; live `kubectl apply/create/wait` evidence still requires deployable operator manifests and a cluster context.
- **Slurm**: `hpc/slurm/submit-experiment.sh --scenario scenarios/factory_bottleneck_v1.yaml --output s3://my-bucket/runs/ --partition gpu --nodes 1` on a scheduler where the wrapper can invoke `sbatch`; use site-supported dry-run/test-only routes when available.
- **Spot resilience scaffold**: Signal handlers trap SIGTERM and write local checkpoint manifests atomically. Full stateful resume depends on Track 22 CLI checkpoint/resume behavior.
- **Telemetry output scaffold**: local file destinations copy Arrow files and write `.sha256` sidecars; `s3://`, `gs://`, and `az://` destinations currently write provider upload manifests for later provider-specific upload execution.
- **Offline validation**: `python cloud/validate_cloud_hpc.py` validates Dockerfile/entrypoint policy, JSON/YAML/text manifest invariants, rendered Kubernetes Job shape, Slurm shell syntax where `bash` is available, local telemetry checksum sidecars, and provider upload manifest schema without live credentials.

## Latest validation evidence

- 2026-05-11: `python cloud/validate_cloud_hpc.py` passed. It covered Docker non-root entrypoint wiring, Kubernetes CRD/sample/operator rendering, AWS/GCP/Azure template shape, Slurm checkpoint/signal wiring, provider docs, local telemetry checksum sidecars, and provider upload manifest generation.

## Runtime evidence boundary (2026-05-11)

Live runtime claims are still blocked by environment availability. Record evidence under `docs/cloud-hpc/runtime-evidence-boundary.md` and only promote Track 39 claims after the commands below are run and their terminal outputs archived.

- **Docker/K8s runtime**: not run in this workspace (no local Docker daemon or Kubernetes cluster context available).
- **Docker live proof required**: `docker build -t kairo-ecs-cli:latest -f docker/Dockerfile .` and `docker run ... run --help` plus a minimal `run` smoke experiment.
- **Kubernetes runtime proof required**: `kubectl apply -f k8s/crd/kairoecs-experiment.yaml`, operator manifest/application, sample CR create, pod completion, and CRD status transition to `Completed` or `Failed`.
- **Slurm runtime proof required**: wrapper canaries for `hpc/slurm/submit-experiment.sh --scenario scenarios/factory_bottleneck_v1.yaml --output /tmp/kairo-ecs-runs --partition gpu --nodes 1` and `hpc/slurm/submit-sweep.sh` on a scheduler where the wrappers can invoke `sbatch`, plus terminal status and checkpoint path evidence.
- **Provider runtime proof required (AWS/GCP/Azure)**: provider-authenticated dry-run or small canary submits plus output/checkpoint terminal validation; no such commands were executed in this scope.

Blockers are explicitly constrained to runtime proof capture, not offline validation. Offline validators remain valid and usable for PR smoke checks and schema review.

## Azure Batch substrate canary -- 2026-05-20

- Accepted partial evidence: a live Azure Batch CPU substrate canary completed in the Azure for Students subscription. The run created a disposable resource group, storage account/container, Batch account, one low-priority `standard_a1_v2` Ubuntu pool node, job `kairos-canary-20260520`, and task `kairos-canary-task-001`.
- Result: the task completed successfully with exit code `0` and returned stdout/stderr. Sanitized evidence is recorded in `docs/cloud-hpc/azure-batch-canary-2026-05-20.md`.
- Boundary: this is substrate evidence only. It does not prove Docker image execution, `kairo-ecs-cli` scenario execution, telemetry output/checksum generation from KairoECS, GPU parity, HPC scaling, or production registry acceptance.
- Deferred by environment: local Docker could not connect to a Docker daemon, so a readable `kairo-ecs-cli` image was not built and pushed from this host. The Batch quota report showed zero GPU/HPC-family quota, so GPU/HPC hardware proof remains blocked in this subscription.
- Hygiene: raw Azure CLI scratch notes and stdout/stderr captures remain under ignored `.azure/`; the repo-tracked evidence stays sanitized and account identifiers remain out of public docs.

## Next-phase decision

Remain `In Review`. Offline validation is complete for this slice, but live Docker, Kubernetes, Slurm, and provider evidence still needs to be captured before any readiness claim.

## Risks and unresolved questions

- Spot instance checkpoint relies on the experiment runner supporting checkpoint/resume subcommands. If Track 22 does not implement these, this track must implement file-based state snapshot as a fallback, which may not be portable across all experiment types.
- Cloud provider emulators (LocalStack, Azurite, GCP emulator) may lag behind production APIs — dry-run validation in CI may pass while production submission fails due to API drift.
- Multi-arch Docker builds on GitHub Actions free-tier runners may be slow for ARM64 emulation. QEMU-based cross-compilation is a workaround but increases build complexity and risk of architecture-specific bugs.
- Kubernetes operator version testing matrix (3 K8s versions) requires a CI environment with kind/minikube. GitHub Actions runner disk space may be tight for multi-version matrix.

## Worker 6 hardening evidence — 2026-05-06

- Added pre-render validation to `k8s/operator/kairoecs_operator.py` for experiment kind, spec shape, non-empty image, positive parallelism, valid storage backend, and non-empty storage path.
- Added aggregate Track 36-40 validator coverage that renders the sample Kubernetes Job and verifies invalid local experiment specs fail before rendering.
- No live Docker, Kubernetes, Slurm, AWS, GCP, or Azure runtime claim was added; current evidence remains offline manifest/policy validation only.

## Contracts changed

No contract changes were recorded by this Conductor hygiene update.


## Tests added

No tests were added by this Conductor hygiene update.


## Known risks

No new risks were introduced by this Conductor hygiene update.


## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.


## Integration notes

No additional integration notes were recorded by this Conductor hygiene update.
## Phase closeout evidence

Pending for the next actual phase closeout. The current blocker is not the offline scaffold; it is the remaining live Docker, Kubernetes, Slurm, and provider-runtime evidence required before readiness claims are defensible. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.

## Review remediation -- 2026-05-17

- Accepted fix: `kairo-ecs-cli` now exposes scaffold-safe `run`, `checkpoint`, and `resume` commands so Track 39 runner artifacts no longer call missing CLI commands.
- Accepted fix: Kubernetes offline rendering now mounts the configured scenario ConfigMap key, passes an explicit output path to the CLI, and can render a status patch for lifecycle proof scaffolding.
- Accepted fix: AWS, GCP, Azure, and Slurm runner templates now pass an explicit `--output` argument to the CLI; the GCP array template now uses numeric sample `taskCount` and `parallelism` fields rather than unrendered strings in typed fields.
- Accepted fix: the offline validator now checks the CLI command surface and warns explicitly when Bash syntax checks are skipped because Git Bash cannot start in this Windows session.
- Deferred by scope: live Docker daemon, Kubernetes controller reconciliation, Slurm `sbatch`, and provider-submitted canary runs remain required before runtime readiness claims.
- Validation: `python cloud\validate_cloud_hpc.py` passed with an explicit Bash-startup warning in this host.

## Review remediation -- 2026-05-18

- Accepted fix: resolved the Track 39 CLI ownership concern by recording explicit Track 22 handoff approval for the existing scaffold-only `run`, `checkpoint`, and `resume` CLI surface in `conductor/tracks/22-experiment-runner-scenario-management/handoff.md`. Track 39 remains a consumer of that surface and does not own production runner semantics.
- Accepted fix: `cloud/gcp/submit-experiment.sh` now renders `cloud/gcp/batch-array.json` with `KAIRO_SWEEP_SIZE` and `KAIRO_PARALLELISM` / `KAIRO_SWEEP_PARALLELISM`, clamping parallelism to the sweep size before submitting to GCP Batch.
- Accepted fix: `cloud/validate_cloud_hpc.py` now writes the inline Kubernetes experiment smoke input under `cloud/validation-work` with explicit cleanup, so it no longer leaves `.tmp/k8s-inline-experiment.json`.
- Accepted fix: `cloud/validate_cloud_hpc.py` now runs a limited static shell fallback when Bash exists but cannot start. The fallback checks shebangs, line endings, quoting, heredoc closure, and common block balance, and warns that it is not equivalent to `bash -n`.
- Validation: `python cloud/validate_cloud_hpc.py` passed with the fallback static shell validation path on this Windows host. Git Bash still failed to start with `couldn't create signal pipe, Win32 error 5`; this does not satisfy live Slurm scheduler proof.
- Cleanup: `cloud/validation-work` was absent after the passing validator run. `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` passed non-strict after rerun outside the sandbox; strict `-RequireCleanWorkingTree` remains inappropriate before commit/push in this shared dirty worktree.

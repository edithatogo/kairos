# Handoff: Track 39 Cloud / HPC Batch Runners

## Summary

Defined the scaffold and offline validation layer for production-scale KairoECS execution. The current verified scope is manifest shape, local rendering, shell syntax, checkpoint/spot policy wiring, and local telemetry checksum behavior. Live Docker builds, Kubernetes cluster reconciliation, Slurm scheduler submission, and AWS/GCP/Azure provider API acceptance still require environment-backed validation before readiness claims.

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
- **K8s operator**: `kubectl apply -f k8s/crd/kairoecs-experiment.yaml && kubectl apply -f k8s/operator/ && kubectl create -f k8s/samples/hello-world-experiment.yaml`
- **Slurm**: `sbatch hpc/slurm/submit-experiment.sh --scenario scenarios/factory_bottleneck_v1.yaml --output s3://my-bucket/runs/ --partition gpu --nodes 1`
- **Spot resilience scaffold**: Signal handlers trap SIGTERM and write local checkpoint manifests atomically. Full stateful resume depends on Track 22 CLI checkpoint/resume behavior.
- **Telemetry output scaffold**: local file destinations copy Arrow files and write `.sha256` sidecars; `s3://`, `gs://`, and `az://` destinations currently write provider upload manifests for later provider-specific upload execution.
- **Offline validation**: `python cloud\validate_cloud_hpc.py` validates Dockerfile/entrypoint policy, JSON/YAML/text manifest invariants, rendered Kubernetes Job shape, Slurm shell syntax where `bash` is available, local telemetry checksum sidecars, and provider upload manifest schema without live credentials.

## Latest validation evidence

- 2026-05-06: `python cloud\validate_cloud_hpc.py` passed after the hardening slice. It covered Docker non-root entrypoint wiring, Kubernetes CRD/sample/operator rendering, AWS/GCP/Azure template shape, Slurm checkpoint/signal wiring, provider docs, local telemetry checksum sidecars, and provider upload manifest generation.

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

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, and next-phase decision here.
# Handoff: Track 39 Cloud / HPC Batch Runners

## Summary

Defined the production-scale execution layer for KairoECS simulations. Established a containerized execution pipeline (multi-arch Docker image wrapping `kairo-ecs-cli`), a Kubernetes operator with a `KairoECSExperiment` CRD, cloud batch provider job definitions (AWS Batch, GCP Batch, Azure Batch), Slurm HPC job submission scripts with checkpoint hooks, spot/preemptible instance resilience via SIGTERM trapping and checkpoint/resume, and a cloud storage telemetry output plugin writing Arrow files with SHA-256 checksums to S3/GCS/Azure Blob.

## Files changed

`conductor/tracks/39-cloud-hpc-batch-runners/spec.md`, `conductor/tracks/39-cloud-hpc-batch-runners/plan.md`, `conductor/tracks/39-cloud-hpc-batch-runners/agent-contract.md`, `conductor/tracks/39-cloud-hpc-batch-runners/risk-register.md`, `conductor/tracks/39-cloud-hpc-batch-runners/test-matrix.md`, `conductor/tracks/39-cloud-hpc-batch-runners/handoff.md`, `docker/Dockerfile`, `docker/docker-bake.hcl`, `docker/entrypoint.sh`, `docker/telemetry-plugin/cloud-output.py`, `k8s/crd/kairoecs-experiment.yaml`, `k8s/operator/`, `cloud/aws/batch-job-definition.yaml`, `cloud/aws/batch-array-template.yaml`, `cloud/aws/submit-experiment.sh`, `cloud/gcp/batch-job.json`, `cloud/gcp/batch-array.json`, `cloud/gcp/submit-experiment.sh`, `cloud/azure/batch-job.json`, `cloud/azure/batch-array.json`, `cloud/azure/submit-experiment.ps1`, `hpc/slurm/submit-experiment.sh`, `hpc/slurm/submit-sweep.sh`, `hpc/slurm/resume.sh`, `docs/cloud-hpc/slurm.md`, `docs/cloud-hpc/aws-batch.md`, `docs/cloud-hpc/gcp-batch.md`, `docs/cloud-hpc/azure-batch.md`, `.github/workflows/cloud-smoke.yml`

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
- **Spot resilience**: Signal handler traps SIGTERM, writes checkpoint atomically. Resume path: `kairo-ecs-cli resume --checkpoint /output/checkpoint.json`
- **Telemetry output**: `docker run -e OUTPUT_BACKEND=s3 -e S3_BUCKET=my-bucket kairo-ecs-cli run ...` writes `<run-id>.arrow` and `<run-id>.arrow.sha256` to S3.

## Risks and unresolved questions

- Spot instance checkpoint relies on the experiment runner supporting checkpoint/resume subcommands. If Track 22 does not implement these, this track must implement file-based state snapshot as a fallback, which may not be portable across all experiment types.
- Cloud provider emulators (LocalStack, Azurite, GCP emulator) may lag behind production APIs — dry-run validation in CI may pass while production submission fails due to API drift.
- Multi-arch Docker builds on GitHub Actions free-tier runners may be slow for ARM64 emulation. QEMU-based cross-compilation is a workaround but increases build complexity and risk of architecture-specific bugs.
- Kubernetes operator version testing matrix (3 K8s versions) requires a CI environment with kind/minikube. GitHub Actions runner disk space may be tight for multi-version matrix.

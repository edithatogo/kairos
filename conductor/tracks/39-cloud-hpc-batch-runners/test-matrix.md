# Test Matrix: Track 39 Cloud / HPC Batch Runners

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---:|---:|---:|---:|
| Track docs exist and render cleanly (spec, plan, agent-contract, risk-register, handoff) | yes | yes | yes |
| `docker/Dockerfile` exists and `docker build` succeeds for native arch | yes | yes | yes |
| `docker build` succeeds for `linux/amd64` and `linux/arm64` (multi-arch) | no | yes | yes |
| `docker run kairo-ecs-cli:<tag> run --help` exits 0 and produces expected CLI help text | yes | yes | yes |
| Container runs a minimal experiment (single tick, single entity) and exits 0 | yes | yes | yes |
| Kubernetes CRD (`kairoecs-experiment.yaml`) is valid against `apiextensions.k8s.io/v1` schema | yes | yes | yes |
| Kubernetes operator smoke test: create experiment CR, verify pod spawns, verify phase transitions to Completed | no | yes | yes |
| Kubernetes operator smoke test: experiment with intentional failure transitions to Failed phase | no | yes | yes |
| Slurm `submit-experiment.sh` syntax-checked with `sbatch --test-only` or equivalent dry-run | yes | yes | yes |
| Slurm job array `submit-sweep.sh` produces correct `#SBATCH --array=0-N` directive for N-1 variants | yes | yes | yes |
| Spot checkpoint/restore test: send SIGTERM to running container, verify checkpoint file written with non-zero size | yes | yes | yes |
| Spot checkpoint/restore test: resume container from checkpoint, verify final state matches uninterrupted run | no | yes | yes |
| S3 telemetry output plugin writes Arrow file and SHA-256 checksum file to S3-compatible storage | no | yes | yes |
| S3 telemetry checksum verification: `sha256sum -c <file>.sha256` passes on downloaded output | no | yes | yes |
| AWS Batch job definition validates against `aws batch register-job-definition` schema | no | yes | yes |
| GCP Batch job definition validates against `gcloud batch jobs submit --dry-run` | no | yes | yes |
| Azure Batch job definition validates against `az batch job create --dry-run` | no | yes | yes |
| `.github/workflows/cloud-smoke.yml` triggers on PRs touching `docker/`, `k8s/`, `cloud/`, `hpc/` | yes | yes | yes |
| Cloud smoke workflow includes Docker build, CLI smoke, and Slurm syntax check | yes | yes | yes |
| Multi-arch build gated behind release tag trigger (not run on every PR) | no | yes | yes |
| Telemetry checksum verification is automated in CI using local S3-compatible service (MinIO/moto) | no | no | yes |

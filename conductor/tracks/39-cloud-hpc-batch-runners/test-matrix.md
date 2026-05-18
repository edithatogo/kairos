# Test Matrix: Track 39 Cloud / HPC Batch Runners

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---:|---:|---:|---:|
| Track docs exist and render cleanly (spec, plan, agent-contract, risk-register, handoff) | yes | yes | yes |
| Offline Track 39 validator covers the CLI command surface, Docker, K8s, cloud batch, Slurm, checkpoint policy, and telemetry sidecars without provider credentials | yes | yes | yes |
| `docker/Dockerfile` exists and `docker build` succeeds for native arch | yes | yes | yes |
| `docker build` succeeds for `linux/amd64` and `linux/arm64` (multi-arch) | no | yes | yes |
| `docker run kairo-ecs-cli:<tag> run --help` exits 0 and produces expected CLI help text | yes | yes | yes |
| Container runs a minimal experiment (single tick, single entity) and exits 0 | yes | yes | yes |
| Kubernetes CRD (`kairoecs-experiment.yaml`) is valid against `apiextensions.k8s.io/v1` schema | yes | yes | yes |
| Kubernetes operator smoke test: create experiment CR, verify pod spawns, verify phase transitions to Completed | no | yes | yes |
| Kubernetes operator smoke test: experiment with intentional failure transitions to Failed phase | no | yes | yes |
| Slurm `submit-experiment.sh` wrapper syntax-checked and prepared for scheduler canary submission via `hpc/slurm/submit-experiment.sh --scenario scenarios/factory_bottleneck_v1.yaml --output /tmp/kairo-ecs-runs --partition gpu --nodes 1` | yes | yes | yes |
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

## Current local validation

- `python cloud/validate_cloud_hpc.py` passed on 2026-05-11 and remains the detailed Track 39 offline validator, including the `kairo-ecs-cli` command surface.
- `python k8s/operator/kairoecs_operator.py --experiment k8s/samples/experiment.json` renders the sample `batch/v1` indexed Job.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\36-streaming-real-time-processing\validate-track36-40.ps1 -SkipCargoTests` runs the aggregate Track 36-40 offline gate and verifies the Kubernetes operator rejects invalid local experiment specs before rendering.
- 2026-05-18 remediation: `python cloud/validate_cloud_hpc.py` passed after the GCP array renderer and validator temp-file cleanup. The validator now checks that GCP array submits require sweep sizing/parallelism inputs and that inline Kubernetes smoke files are scoped under `cloud/validation-work`.
- 2026-05-18 hardening: when Bash is found but cannot start on Windows, `python cloud/validate_cloud_hpc.py` now runs a limited static shell fallback for quoting, heredoc closure, shebangs, line endings, and common block balance. This fallback is explicitly reported as not equivalent to `bash -n`; live Slurm and provider runtime proof remain pending under `docs/cloud-hpc/runtime-evidence-boundary.md`.

## Runtime validation blockers (2026-05-11)

| Scope | Command family | Current status | Blocker |
|---|---|---|---|
| Docker runtime smoke | `docker build`, `docker run kairo-ecs-cli:latest ...` | pending | No Docker daemon and runtime execution in this workspace |
| Kubernetes operator integration smoke | `kubectl` apply/create/wait for CR/DCR status transitions | pending | No local Kubernetes cluster or operator deployment context |
| Slurm single-job + sweep smoke | `hpc/slurm/submit-experiment.sh --scenario scenarios/factory_bottleneck_v1.yaml --output /tmp/kairo-ecs-runs --partition gpu --nodes 1` and `KAIRO_SCENARIO_PREFIX=... KAIRO_OUTPUT_URI=... KAIRO_SWEEP_SIZE=... hpc/slurm/submit-sweep.sh` on a scheduler where the wrappers can invoke `sbatch` | pending | No Slurm scheduler context available |
| AWS Batch acceptance | rendered template + canary submit/describe | pending | No AWS credentials/account context |
| GCP Batch acceptance | rendered template + canary submit/get | pending | No GCP credentials/account context |
| Azure Batch acceptance | rendered template + canary job/task create/get | pending | No Azure credentials/account context |

See `docs/cloud-hpc/runtime-evidence-boundary.md` for the canonical runtime blocker ledger before any readiness claim.

## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.

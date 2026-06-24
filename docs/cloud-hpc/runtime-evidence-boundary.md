# Track 39 Runtime Evidence Boundary

Use this file as the single source for live runtime expectations before any "ready" claim for
Docker, Kubernetes, Slurm, and cloud-batch execution.

Last updated: 2026-05-20 (Australia/Sydney)

## Runtime state

| Runtime scope | Offline equivalent | Live command requirement | Status | Status notes |
|---|---|---|---|---|
| Docker image build and CLI smoke | `python cloud/validate_cloud_hpc.py` | `docker build -t kairo-ecs-cli:latest -f docker/Dockerfile .`<br>`docker run --rm kairo-ecs-cli:latest --help`<br>`docker run --rm kairo-ecs-cli:latest run --scenario ...` | pending | Not executed in this workspace (no Docker runtime executed). |
| Docker checkpoint resume behavior | Checkpoint manifest and resume precondition in docs | SIGTERM replay with manifest re-use in a tracked canary run | pending | No tracked container checkpoint-resume run completed with checkpoint integrity assertion. |
| Kubernetes operator smoke | `python k8s/operator/kairoecs_operator.py --experiment k8s/samples/experiment.json` | `kubectl apply -f k8s/crd/kairoecs-experiment.yaml`<br>`kubectl apply -f <operator manifests>`<br>`kubectl create -f k8s/samples/...`<br>`kubectl wait ...` and CRD phase checks | pending | Not executed in this workspace (no local cluster context). |
| Slurm single-job submission | Slurm wrapper and generated-script syntax checks in validator | `hpc/slurm/submit-experiment.sh --scenario scenarios/factory_bottleneck_v1.yaml --output /tmp/kairo-ecs-runs --partition gpu --nodes 1` on a scheduler where the wrapper can invoke `sbatch` (or equivalent canary submit) | pending | No live scheduler context in this workspace. |
| Slurm sweep/job-array behavior | Slurm wrapper and generated-array script syntax checks in validator | `KAIRO_SCENARIO_PREFIX=scenarios/factory_bottleneck_v1 KAIRO_OUTPUT_URI=s3://example/runs KAIRO_SWEEP_SIZE=8 hpc/slurm/submit-sweep.sh` on a scheduler where the wrapper can invoke `sbatch` (or equivalent canary array submit) | pending | No live scheduler context in this workspace. |
| AWS Batch acceptance | `python cloud/validate_cloud_hpc.py` | rendered template render + canary in AWS sandbox/test account | pending | No AWS credentials/CLI run in this workspace. |
| GCP Batch acceptance | `python cloud/validate_cloud_hpc.py` | rendered template render + canary in GCP test project | pending | No GCP credentials/CLI run in this workspace. |
| Azure Batch acceptance | `python cloud/validate_cloud_hpc.py` | rendered template render + canary job/task in Azure Batch | partial | Live Azure Batch CPU substrate canary succeeded on 2026-05-20; see `azure-batch-canary-2026-05-20.md`. KairoECS container/scenario execution and GPU/HPC hardware proof remain pending. |

## Closeout blockers (current)

- Live Docker proof is missing from this slice.
- Live Kubernetes operator proof is missing from this slice.
- Live Slurm submission proof is missing from this slice.
- Live provider acceptance proof is missing for AWS/GCP from this slice.
- Azure has CPU Batch substrate evidence only; KairoECS container/scenario
  execution and GPU/HPC hardware proof remain missing.

## Latest local environment probe

Read-only probe on 2026-06-18:

- `docker`: not found on `PATH`; no local Docker canary can be run from this shell.
- `kubectl`: not found on `PATH`; no Kubernetes context can be checked from this shell.
- `sbatch`: not found on `PATH`; no Slurm scheduler canary can be submitted from this shell.
- `aws`: not found on `PATH`; no AWS Batch identity or canary can be checked from this shell.
- `gcloud`: CLI has a configured project, but no GCP Batch mutation was run.
- `az`: Azure for Students subscription is enabled; no Azure mutation was run in this probe.

Read-only Docker preflight on 2026-06-24:

- `node scripts/validation/validate-docker-runtime-preflight.mjs`: records Docker
  CLI/daemon readiness when available and returns an explicit blocker otherwise.
- `node scripts/validation/validate-docker-runtime-preflight.mjs --require-docker`:
  strict form for CI or live Docker canaries that must fail when Docker is not
  usable.

Read-only Kubernetes, Slurm, and cloud CLI preflight on 2026-06-24:

- `node scripts/validation/validate-external-runtime-preflight.mjs`: records
  `kubectl`, `sbatch`, `aws`, `gcloud`, and `az` probe availability and
  distinguishes missing CLI, sandbox-blocked spawn, and failed probe states.
- `node scripts/validation/validate-external-runtime-preflight.mjs --require-all`:
  strict form for a live runtime canary host that must have every external CLI
  available before Kubernetes, Slurm, or provider acceptance evidence can be
  captured.
- Passing this preflight is not live runtime proof. It only proves that the
  runner can start the external control-plane clients needed by the live gates.

## Evidence format policy

- Runtime evidence must include: command used, runner/cluster/account/region/partition, job identifiers, final status, and output/checkpoint artifact references.
- Checkpoint/resume evidence must include: manifest path, checksum/hash (if present), and matching terminal state across interrupted vs uninterrupted runs.
- Until all required runtime evidence is recorded, Track 39 remains partial-scoped and should not be marked complete.

## Conservative reporting rule

For all runtime scope items above, this track may describe offline validation as:

> “validated for scaffold and wiring only”

and must **not** claim production readiness without the pending live evidence records.

# Track 54: Slurm, Container & Cloud HPC Runtime Acceptance

## Purpose

Provide live runtime acceptance for Slurm, containers, MPI/GPU job templates,
and AWS/GCP/Azure Batch canaries so KairoECS can prove scheduler execution
instead of publishing offline-only cloud/HPC scaffolds.

## Maturity

Spec Approved planning track. No new live scheduler, container, or provider
runtime proof is claimed by this artifact.

## Inputs

- Track 39 cloud/HPC runner scaffolds.
- Track 43 publication and runtime acceptance rules.
- Track 49 distributed launch contracts.
- Track 52 GPU runtime requirements.
- Evidence manifest from Track 46.

## Outputs

- Slurm single-job and array-job scripts.
- Container build/run evidence with digest and checksum outputs.
- MPI and GPU scheduler job templates.
- AWS, GCP, and Azure Batch canaries with output artifacts.
- Protected publication and runtime acceptance handoff to release tracks.

## Owned paths

- `docker/`
- `k8s/`
- `hpc/slurm/`
- `cloud/`
- `docs/cloud-hpc/`
- `conductor/tracks/54-slurm-container-cloud-hpc-runtime-acceptance/`

## Blocked paths

- MPI/gRPC runtime implementation owned by Track 49.
- GPU runtime implementation owned by Track 52.
- Final scaling certification owned by Track 55.

## Dependencies

Tracks 39, 43, 49, and 52.

## Parallel-safe tracks

Track 55 may draft scaling runbooks after this track defines scheduler evidence
locations and artifact collection.

## Acceptance criteria

- Docker image builds and runs a KairoECS scenario with checksum output.
- Kubernetes workload launches and records pod/job completion evidence.
- Slurm single-job and array-job runs complete with scheduler job IDs.
- AWS, GCP, and Azure Batch canaries run KairoECS scenarios or record explicit
  provider quota blockers.
- GPU and MPI job templates exercise real Track 49/52 runtime paths.

## Quality gates

- `container-runtime-kairos`
- `kubernetes-runtime-kairos`
- `slurm-runtime-kairos`
- `provider-batch-canary`
- `scheduler-runtime-evidence`
- `phase-closeout-check`

## Release implications

This track gates production cloud/HPC runtime acceptance and any publication
that implies container, Slurm, Kubernetes, or provider execution readiness.

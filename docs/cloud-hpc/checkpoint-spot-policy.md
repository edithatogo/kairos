# Checkpoint and Spot Interruption Policy

## Offline validator scope

`python cloud\validate_cloud_hpc.py` checks that the Track 39 artifacts expose the expected checkpoint and interruption wiring without requiring Docker, Kubernetes, Slurm, or cloud credentials. It validates:

- the Docker entrypoint traps `TERM`/`INT`, writes checkpoint manifests atomically, and attempts resume when a checkpoint manifest exists;
- Slurm generated scripts request `--signal=B:SIGTERM@120`, create a local checkpoint directory, and call `kairo-ecs-cli checkpoint` from the signal trap;
- Kubernetes rendered Jobs carry storage and checkpoint environment variables;
- AWS, GCP, and Azure batch templates include output and checkpoint wiring; and
- telemetry copies write SHA-256 sidecars locally and provider upload manifests for `s3://`, `gs://`, and `az://` destinations.

This proves manifest shape and local policy wiring. It does not prove that a provider accepts the rendered template, that a real preemptible VM delivers the signal in time, or that `kairo-ecs-cli checkpoint`/`resume` preserves domain state for every experiment type.

## Checkpoint location

Checkpoint directories must be local or shared POSIX filesystem paths visible to the running process. Do not default checkpoint state to `s3://`, `gs://`, or `az://` URIs. Cloud object storage remains the telemetry/output destination; checkpoint upload can be added later by a provider-specific sync step after local checkpoint integrity is verified.

The default container checkpoint path is:

```bash
${KAIRO_OUTPUT_DIR:-/var/lib/kairo/output}/checkpoints
```

The default Slurm checkpoint path is:

```bash
${TMPDIR:-/tmp}/kairo/checkpoints
```

For HPC clusters, set `KAIRO_CHECKPOINT_DIR` to a shared scratch path if resumed jobs may start on different nodes.

## Live provider validation

Before claiming provider readiness, run the provider-native validation for the rendered templates:

- AWS Batch: register the job definition in a non-production account or submit a canary job to a test queue and cancel/describe it.
- GCP Batch: submit a canary job with `cloud/gcp/batch-job.json` in a test project and inspect terminal status.
- Azure Batch: create a canary job and task in a test Batch account and inspect task exit code and output files.
- Slurm: run `sbatch --test-only` if the site supports it, otherwise submit a one-task canary to a short/debug partition.

Record provider, region/cluster, command, job id, terminal status, and output/checksum evidence in the Track 39 handoff.

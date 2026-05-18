# GCP Batch

See `runtime-evidence-boundary.md` for required live evidence and pending blocker status before any production-readiness claim.

`cloud/gcp/batch-job.json` defines a single task group using the KairoECS container image. `cloud/gcp/batch-array.json` is the array-run template; `cloud/gcp/submit-experiment.sh` renders its `taskCount` and `parallelism` from `KAIRO_SWEEP_SIZE` and `KAIRO_PARALLELISM` / `KAIRO_SWEEP_PARALLELISM` before submitting it. Use workload identity or a service account with write access to the configured GCS output prefix.

## Offline validation

Run `python cloud\validate_cloud_hpc.py` from the repository root. The offline validator parses the GCP JSON templates and checks image placeholders, `run --scenario` command wiring, output/checkpoint environment variables, task count shape, array index parameterization, and array submitter rendering for sweep sizing and parallelism.

## Live validation

The offline check is not a GCP API validation. Before marking GCP Batch ready, submit a small canary job in a test project and region, then record the job name, location, terminal status, and output/checksum evidence.

### Runtime evidence status

- This doc is paired with `runtime-evidence-boundary.md` for pending live proof blockers.

Required permissions:

- `batch.jobs.create`
- `batch.jobs.get`
- `storage.objects.create`

Submit a run:

```bash
GCP_BATCH_LOCATION=us-central1 GCP_BATCH_CONFIG=cloud/gcp/batch-job.json cloud/gcp/submit-experiment.sh kairo-run-001
```

Submit a parameter sweep:

```bash
KAIRO_SWEEP_SIZE=24 KAIRO_PARALLELISM=4 GCP_BATCH_LOCATION=us-central1 GCP_BATCH_CONFIG=cloud/gcp/batch-array.json cloud/gcp/submit-experiment.sh kairo-sweep-001
```

# GCP Batch

`cloud/gcp/batch-job.json` defines a single task group using the KairoECS container image. Use workload identity or a service account with write access to the configured GCS output prefix.

## Offline validation

Run `python cloud\validate_cloud_hpc.py` from the repository root. The offline validator parses the GCP JSON templates and checks image placeholders, `run --scenario` command wiring, output/checkpoint environment variables, task count shape, and array index parameterization.

## Live validation

The offline check is not a GCP API validation. Before marking GCP Batch ready, submit a small canary job in a test project and region, then record the job name, location, terminal status, and output/checksum evidence.

Required permissions:

- `batch.jobs.create`
- `batch.jobs.get`
- `storage.objects.create`

Submit a run:

```bash
GCP_BATCH_LOCATION=us-central1 GCP_BATCH_CONFIG=cloud/gcp/batch-job.json cloud/gcp/submit-experiment.sh kairo-run-001
```

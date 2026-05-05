# GCP Batch

`cloud/gcp/batch-job.json` defines a single task group using the KairoECS container image. Use workload identity or a service account with write access to the configured GCS output prefix.

Required permissions:

- `batch.jobs.create`
- `batch.jobs.get`
- `storage.objects.create`

Submit a run:

```bash
GCP_BATCH_LOCATION=us-central1 GCP_BATCH_CONFIG=cloud/gcp/batch-job.json cloud/gcp/submit-experiment.sh kairo-run-001
```

#!/usr/bin/env bash
set -euo pipefail

job_name="${1:?job name required}"
location="${GCP_BATCH_LOCATION:-us-central1}"
config="${GCP_BATCH_CONFIG:-cloud/gcp/batch-job.json}"

gcloud batch jobs submit "$job_name" \
  --location "$location" \
  --config "$config"

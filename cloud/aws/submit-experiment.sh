#!/usr/bin/env bash
set -euo pipefail

job_name="${1:?job name required}"
job_queue="${AWS_BATCH_JOB_QUEUE:?AWS_BATCH_JOB_QUEUE required}"
job_definition="${AWS_BATCH_JOB_DEFINITION:-kairo-ecs-cli}"
scenario="${KAIRO_SCENARIO:?KAIRO_SCENARIO required}"
output_uri="${KAIRO_OUTPUT_URI:?KAIRO_OUTPUT_URI required}"

aws batch submit-job \
  --job-name "$job_name" \
  --job-queue "$job_queue" \
  --job-definition "$job_definition" \
  --parameters "scenario=$scenario,output_uri=$output_uri"

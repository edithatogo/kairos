#!/usr/bin/env bash
set -euo pipefail

job_name="${1:?job name required}"
location="${GCP_BATCH_LOCATION:-us-central1}"
config="${GCP_BATCH_CONFIG:-cloud/gcp/batch-job.json}"

rendered_config=""
if [[ "${config}" == *"batch-array.json" ]]; then
  sweep_size="${KAIRO_SWEEP_SIZE:?KAIRO_SWEEP_SIZE required for GCP array jobs}"
  parallelism="${KAIRO_PARALLELISM:-${KAIRO_SWEEP_PARALLELISM:-$sweep_size}}"
  if [[ ! "$sweep_size" =~ ^[1-9][0-9]*$ ]]; then
    echo "KAIRO_SWEEP_SIZE must be a positive integer" >&2
    exit 1
  fi
  if [[ ! "$parallelism" =~ ^[1-9][0-9]*$ ]]; then
    echo "KAIRO_PARALLELISM must be a positive integer" >&2
    exit 1
  fi

  rendered_config="$(mktemp "${TMPDIR:-/tmp}/kairo-gcp-batch-array.XXXXXX.json")"
  python - "$config" "$rendered_config" "$sweep_size" "$parallelism" <<'PY'
import json
import sys
from pathlib import Path

source = Path(sys.argv[1])
target = Path(sys.argv[2])
sweep_size = int(sys.argv[3])
parallelism = int(sys.argv[4])

config = json.loads(source.read_text(encoding="utf-8"))
task_group = config["taskGroups"][0]
task_group["taskCount"] = sweep_size
task_group["parallelism"] = min(parallelism, sweep_size)
target.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
PY
  config="$rendered_config"
  trap 'rm -f "$rendered_config"' EXIT
fi

gcloud batch jobs submit "$job_name" \
  --location "$location" \
  --config "$config"

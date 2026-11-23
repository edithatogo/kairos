#!/usr/bin/env bash
set -euo pipefail

scenario_prefix="${KAIRO_SCENARIO_PREFIX:?KAIRO_SCENARIO_PREFIX required}"
seed_manifest_prefix="${KAIRO_SEED_MANIFEST_PREFIX:-}"
output_uri="${KAIRO_OUTPUT_URI:?KAIRO_OUTPUT_URI required}"
sweep_size="${KAIRO_SWEEP_SIZE:?KAIRO_SWEEP_SIZE required}"
partition="${SLURM_PARTITION:-cpu}"

if ! [[ "$sweep_size" =~ ^[1-9][0-9]*$ && $((sweep_size)) == "$sweep_size" ]]; then
  echo "KAIRO_SWEEP_SIZE must be a positive integer in range" >&2
  exit 2
fi
last_index=$((sweep_size - 1))
script="$(mktemp "${TMPDIR:-/tmp}/kairo-ecs-sweep.XXXXXX.sh")"
trap 'rm -f -- "$script"' EXIT
cat > "$script" <<'SLURM'
#!/usr/bin/env bash
#SBATCH --job-name=kairo-ecs-sweep
#SBATCH --signal=B:SIGTERM@120
#SBATCH --output=kairo-ecs-sweep-%A-%a.out

set -euo pipefail
scenario_prefix="$1"
output_uri="$2"
seed_manifest_prefix="$3"
variant="$SLURM_ARRAY_TASK_ID"
export KAIRO_OUTPUT_URI="${output_uri}/variant-${variant}"
export KAIRO_CHECKPOINT_DIR="${KAIRO_CHECKPOINT_DIR:-${TMPDIR:-/tmp}/kairo/checkpoints/variant-${variant}}"
mkdir -p "$KAIRO_CHECKPOINT_DIR"

on_term() {
  kairo-ecs-cli checkpoint --output "$KAIRO_CHECKPOINT_DIR" || true
}
trap on_term TERM

if [[ -n "$seed_manifest_prefix" ]]; then
  kairo-ecs-cli run --scenario "${scenario_prefix}/variant-${variant}.scenario.toml" \
    --seed-manifest "${seed_manifest_prefix}/variant-${variant}.seeds.toml" --output "$KAIRO_OUTPUT_URI"
else
  kairo-ecs-cli run --scenario "${scenario_prefix}/variant-${variant}.yaml" --output "$KAIRO_OUTPUT_URI"
fi
SLURM

sbatch --partition="$partition" --array="0-${last_index}" -- "$script" "$scenario_prefix" "$output_uri" "$seed_manifest_prefix"

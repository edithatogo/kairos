#!/usr/bin/env bash
set -euo pipefail

scenario_prefix="${KAIRO_SCENARIO_PREFIX:?KAIRO_SCENARIO_PREFIX required}"
output_uri="${KAIRO_OUTPUT_URI:?KAIRO_OUTPUT_URI required}"
sweep_size="${KAIRO_SWEEP_SIZE:?KAIRO_SWEEP_SIZE required}"
partition="${SLURM_PARTITION:-cpu}"

last_index=$((sweep_size - 1))
script="$(mktemp "${TMPDIR:-/tmp}/kairo-ecs-sweep.XXXXXX.sh")"
cat > "$script" <<SLURM
#!/usr/bin/env bash
#SBATCH --job-name=kairo-ecs-sweep
#SBATCH --partition=${partition}
#SBATCH --array=0-${last_index}
#SBATCH --signal=B:SIGTERM@120
#SBATCH --output=kairo-ecs-sweep-%A-%a.out

set -euo pipefail
variant="\${SLURM_ARRAY_TASK_ID}"
export KAIRO_OUTPUT_URI="${output_uri}/variant-\${variant}"
export KAIRO_CHECKPOINT_DIR="\${KAIRO_CHECKPOINT_DIR:-${TMPDIR:-/tmp}/kairo/checkpoints/variant-\${variant}}"
mkdir -p "\$KAIRO_CHECKPOINT_DIR"

on_term() {
  kairo-ecs-cli checkpoint --output "\$KAIRO_CHECKPOINT_DIR" || true
}
trap on_term TERM

kairo-ecs-cli run --scenario "${scenario_prefix}/variant-\${variant}.yaml"
SLURM

sbatch "$script"

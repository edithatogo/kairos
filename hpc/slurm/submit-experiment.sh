#!/usr/bin/env bash
set -euo pipefail

scenario=""
output=""
partition="${SLURM_PARTITION:-cpu}"
nodes="1"
image="${KAIRO_IMAGE:-kairo-ecs-cli:latest}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario) scenario="$2"; shift 2 ;;
    --output) output="$2"; shift 2 ;;
    --partition) partition="$2"; shift 2 ;;
    --nodes) nodes="$2"; shift 2 ;;
    --image) image="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[[ -n "$scenario" ]] || { echo "--scenario is required" >&2; exit 2; }
[[ -n "$output" ]] || { echo "--output is required" >&2; exit 2; }

script="$(mktemp "${TMPDIR:-/tmp}/kairo-ecs-slurm.XXXXXX.sh")"
cat > "$script" <<SLURM
#!/usr/bin/env bash
#SBATCH --job-name=kairo-ecs
#SBATCH --partition=${partition}
#SBATCH --nodes=${nodes}
#SBATCH --ntasks=1
#SBATCH --signal=B:SIGTERM@120
#SBATCH --output=kairo-ecs-%j.out

set -euo pipefail
export KAIRO_OUTPUT_URI="${output}"
export KAIRO_CHECKPOINT_DIR="\${KAIRO_CHECKPOINT_DIR:-${output}/checkpoints}"

on_term() {
  kairo-ecs-cli checkpoint --output "\$KAIRO_CHECKPOINT_DIR" || true
}
trap on_term TERM

if command -v apptainer >/dev/null 2>&1; then
  apptainer exec "docker://${image}" kairo-ecs-cli run --scenario "${scenario}"
else
  kairo-ecs-cli run --scenario "${scenario}"
fi
SLURM

sbatch "$script"

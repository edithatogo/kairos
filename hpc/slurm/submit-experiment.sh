#!/usr/bin/env bash
set -euo pipefail

scenario=""
seed_manifest=""
output=""
partition="${SLURM_PARTITION:-cpu}"
nodes="1"
image="${KAIRO_IMAGE:-kairo-ecs-cli:latest}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario) scenario="$2"; shift 2 ;;
    --seed-manifest) seed_manifest="${2:?--seed-manifest requires a path}"; shift 2 ;;
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
trap 'rm -f -- "$script"' EXIT
cat > "$script" <<'SLURM'
#!/usr/bin/env bash
#SBATCH --job-name=kairo-ecs
#SBATCH --ntasks=1
#SBATCH --signal=B:SIGTERM@120
#SBATCH --output=kairo-ecs-%j.out

set -euo pipefail
scenario="$1"
output="$2"
image="$3"
seed_manifest="$4"
export KAIRO_OUTPUT_URI="$output"
export KAIRO_CHECKPOINT_DIR="${KAIRO_CHECKPOINT_DIR:-${TMPDIR:-/tmp}/kairo/checkpoints}"
mkdir -p "$KAIRO_CHECKPOINT_DIR"

on_term() {
  kairo-ecs-cli checkpoint --output "$KAIRO_CHECKPOINT_DIR" || true
}
trap on_term TERM

run=(run --scenario "$scenario" --output "$output")
if [[ -n "$seed_manifest" ]]; then
  run+=(--seed-manifest "$seed_manifest")
fi
if command -v apptainer >/dev/null 2>&1; then
  apptainer exec "docker://${image}" kairo-ecs-cli "${run[@]}"
else
  kairo-ecs-cli "${run[@]}"
fi
SLURM

sbatch --partition="$partition" --nodes="$nodes" -- "$script" "$scenario" "$output" "$image" "$seed_manifest"

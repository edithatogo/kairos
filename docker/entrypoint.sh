#!/usr/bin/env bash
set -euo pipefail

checkpoint_dir="${KAIRO_CHECKPOINT_DIR:-${KAIRO_OUTPUT_DIR:-/var/lib/kairo/output}/checkpoints}"
checkpoint_manifest="${KAIRO_CHECKPOINT_MANIFEST:-${checkpoint_dir}/checkpoint-manifest.json}"

mkdir -p "$checkpoint_dir"

child_pid=""

write_checkpoint() {
  local completed_tick="${KAIRO_LAST_COMPLETED_TICK:-unknown}"
  local tmp_file="${checkpoint_manifest}.tmp"
  cat > "$tmp_file" <<JSON
{
  "schema": "kairo.ecs.checkpoint.v1",
  "state": "interrupted",
  "last_completed_tick": "${completed_tick}",
  "arrow_output_position": "${KAIRO_ARROW_OUTPUT_POSITION:-unknown}",
  "written_at_unix": "$(date +%s)"
}
JSON
  mv "$tmp_file" "$checkpoint_manifest"
}

on_term() {
  write_checkpoint
  if [[ -n "$child_pid" ]] && kill -0 "$child_pid" 2>/dev/null; then
    kill -TERM "$child_pid"
    wait "$child_pid" || true
  fi
  exit 143
}

trap on_term TERM INT

if [[ -f "$checkpoint_manifest" ]] && [[ "${KAIRO_DISABLE_RESUME:-0}" != "1" ]]; then
  set -- resume --checkpoint "$checkpoint_manifest" "$@"
fi

kairo-ecs-cli "$@" &
child_pid="$!"
wait "$child_pid"

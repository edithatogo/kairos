#!/usr/bin/env bash
set -euo pipefail

checkpoint="${1:?checkpoint manifest path required}"
output="${KAIRO_OUTPUT_URI:?KAIRO_OUTPUT_URI required}"

kairo-ecs-cli resume --checkpoint "$checkpoint" --output "$output"

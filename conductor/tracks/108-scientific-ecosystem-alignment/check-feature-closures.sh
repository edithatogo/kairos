#!/usr/bin/env bash
# Run from the repository root. Inspect normal dependencies of the selected target.
set -euo pipefail

packages() {
  cargo tree --locked --edges normal --prefix none \
    --format '{p}' "$@" | awk '{print $1}' | sort -u
}

reject_polars() {
  if grep -Eq '^polars($|-)' <<< "$1"; then
    echo "Unexpected Polars dependency in $2" >&2
    return 1
  fi
}

is_minimal() {
  ! grep -Eq '^(arrow-schema|polars($|-))' <<< "$1"
}

for package in kairo-ecs-core kairo-ecs-types kairo-ecs-wasm kairo-ecs-arrow; do
  for profile in default minimal; do
    args=(-p "$package")
    if [[ $profile == minimal ]]; then
      args+=(--no-default-features)
    fi
    if [[ $package == kairo-ecs-wasm ]]; then
      args+=(--target wasm32-unknown-unknown)
    fi
    closure=$(packages "${args[@]}")
    grep -Fxq "$package" <<< "$closure"
    if ! is_minimal "$closure"; then
      echo "Optional scientific dependency in $package ($profile)" >&2
      exit 1
    fi
  done
done

schema=$(packages --no-default-features -p kairo-ecs-arrow --features arrow-native)
grep -Fxq arrow-schema <<< "$schema"
reject_polars "$schema" arrow-native
if is_minimal "$schema"; then
  echo "Isolation control accepted the Arrow schema graph" >&2
  exit 1
fi

native=$(packages --no-default-features -p kairo-ecs-arrow --features polars-native)
for package in polars polars-arrow; do
  grep -Fxq "$package" <<< "$native"
done
# Exercise the same rejection path with a real graph that includes Polars.
if reject_polars "$native" polars-native 2>/dev/null; then
  echo "Isolation control accepted the Polars graph" >&2
  exit 1
fi
echo "Minimal, Arrow schema and Polars native dependency closures verified."

#!/usr/bin/env bash
set -euo pipefail

stage="$(mktemp -d "$RUNNER_TEMP/kairo-onnx.XXXXXX")"
archive="$stage/onnxruntime-1.32.0-linux-x64.whl"
curl --fail --silent --show-error --location \
  'https://files.pythonhosted.org/packages/94/b5/cd55e7f1dfcf2e5fbf3a6915a8c6b4cf81df091e81de2db40e58357ffba4/onnxruntime-1.32.0-cp314-cp314-manylinux_2_28_x86_64.whl' \
  --output "$archive"
printf '%s  %s\n' 48b878f9551d465aa79ee94ecaf49a1f6b5d27f2975efa775ded374db14d0af1 "$archive" | sha256sum -c -
for library in libonnxruntime.so.1.32.0 libonnxruntime_providers_shared.so; do
  unzip -p "$archive" "onnxruntime/capi/$library" > "$stage/$library"
  test -s "$stage/$library"
  sha256sum "$stage/$library"
done

printf 'KAIRO_TEST_ONNX_LIBRARY=%s\n' "$stage/libonnxruntime.so.1.32.0" >> "$GITHUB_ENV"

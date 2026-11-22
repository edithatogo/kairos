#!/usr/bin/env bash
set -euo pipefail

stage="$(mktemp -d "$RUNNER_TEMP/kairo-hdf5.XXXXXX")"
archive="$stage/hdf5-2.2.0-ubuntu-2404_gcc.tar.gz"
curl --fail --silent --show-error --location \
  https://github.com/HDFGroup/hdf5/releases/download/2.2.0/hdf5-2.2.0-ubuntu-2404_gcc.tar.gz \
  --output "$archive"
printf '%s  %s\n' efa520a59202830a0d48ac7da1312314507ec1fcc40c744cf7b1c37a5edfaaeb "$archive" | sha256sum --check
tar -xzf "$archive" -C "$stage"
tar -xzf "$stage/hdf5/HDF5-2.2.0-Linux.tar.gz" -C "$stage"
rm -- "$archive" "$stage/hdf5/HDF5-2.2.0-Linux.tar.gz"
install="$stage/HDF5-2.2.0-Linux/HDF_Group/HDF5/2.2.0"
library_path="$install/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
LD_LIBRARY_PATH="$library_path" "$install/bin/h5dump" --version | grep -Fx 'h5dump: Version 2.2.0'
printf 'HDF5_DIR=%s\nLD_LIBRARY_PATH=%s\nKAIROS_H5DUMP=%s\n' \
  "$install" "$library_path" "$install/bin/h5dump" >> "$GITHUB_ENV"

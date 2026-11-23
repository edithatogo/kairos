#!/usr/bin/env bash
set -euo pipefail

stage="$(mktemp -d "$RUNNER_TEMP/kairo-adios2.XXXXXX")"
archive="$stage/adios2-2.12.1.tar.gz"
source="$stage/source"
install="$stage/install"
curl --fail --silent --show-error --location \
  https://codeload.github.com/ornladios/ADIOS2/legacy.tar.gz/f5267290f06980acaecaf54688d0980958eb86bf \
  --output "$archive"
printf '%s  %s\n' 95951927f6ddfdd7fc9c8c284f87a1dec1440b6d46f226881a590bead7e87b4c "$archive" | sha256sum --check
mkdir "$source"
tar -xzf "$archive" -C "$source" --strip-components=1

cmake -S "$source" -B "$stage/build" \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$install" -DCMAKE_INSTALL_LIBDIR=lib \
  -DBUILD_SHARED_LIBS=ON -DBUILD_TESTING=OFF -DADIOS2_BUILD_EXAMPLES=OFF \
  -DADIOS2_USE_MPI=OFF -DADIOS2_USE_Python=OFF -DADIOS2_USE_Fortran=OFF \
  -DADIOS2_USE_HDF5=OFF -DADIOS2_USE_SST=OFF -DADIOS2_USE_DataMan=OFF
cmake --build "$stage/build" --parallel 2
cmake --install "$stage/build"

lib="$install/lib"
test -f "$lib/libadios2_c.so"
library_path="$lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
LD_LIBRARY_PATH="$library_path" "$install/bin/bpls" --version | grep -Fx '2.12.1'
printf 'ADIOS2_LIB_DIR=%s\nLD_LIBRARY_PATH=%s\nKAIROS_BPLS=%s\n' \
  "$lib" "$library_path" "$install/bin/bpls" >> "$GITHUB_ENV"

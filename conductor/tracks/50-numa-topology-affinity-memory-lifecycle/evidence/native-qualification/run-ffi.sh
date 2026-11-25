#!/usr/bin/env bash
set -euo pipefail
# Run from the repo root. The first argument is the exact built macOS dylib.
source_library=$1
install_dir=$2
evidence=$(cd "$(dirname "$0")" && pwd)
mkdir "$install_dir"
library="$install_dir/$(basename "$source_library")"
cp "$source_library" "$library"
cp include/kairo_ecs.h "$install_dir/kairo_ecs.h"
clang -std=c11 -O2 -Wall -Wextra -Werror -I "$install_dir" -DTRACK50_STANDALONE \
    "$evidence/c-data-lifetime.c" "$library" -Wl,-rpath,"$install_dir" -o "$install_dir/c-data-lifetime"
clang -std=c11 -O2 -Wall -Wextra -Werror -I "$install_dir" -dynamiclib \
    "$evidence/c-data-lifetime.c" "$library" -Wl,-rpath,"$install_dir" -o "$install_dir/libc-data-lifetime.dylib"
original_id=$(otool -D "$library" | sed -n '2p')
new_id="@rpath/$(basename "$library")"
for consumer in "$install_dir/c-data-lifetime" "$install_dir/libc-data-lifetime.dylib"; do
    install_name_tool -change "$original_id" "$new_id" "$consumer"
    otool -L "$consumer" | grep -F "$new_id" > /dev/null
    codesign --force --sign - "$consumer"
done
cmp "$source_library" "$library"
shasum -a 256 "$library" "$install_dir/kairo_ecs.h" "$install_dir/c-data-lifetime" "$install_dir/libc-data-lifetime.dylib"
DYLD_PRINT_LIBRARIES=1 "$install_dir/c-data-lifetime" 2> "$install_dir/c-load.log"
grep -F "$library" "$install_dir/c-load.log" > /dev/null
DYLD_PRINT_LIBRARIES=1 .venv/bin/python -I "$evidence/c-data-lifetime.py" "$install_dir/libc-data-lifetime.dylib" 2> "$install_dir/python-load.log"
grep -F "$library" "$install_dir/python-load.log" > /dev/null

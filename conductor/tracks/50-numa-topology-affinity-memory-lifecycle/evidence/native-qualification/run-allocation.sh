#!/usr/bin/env bash
set -euo pipefail
probe=$1
for trial in {0..9}; do
    for width in 256 2048; do
        if ((trial % 2 == 0)); then
            order=("growing serial" "reserved parallel" "reserved serial" "growing parallel")
        else
            order=("growing parallel" "reserved serial" "reserved parallel" "growing serial")
        fi
        for case in "${order[@]}"; do
            read -r setup mode <<< "$case"
            "$probe" "$setup" "$mode" "$width" | sed "s/^/$trial,/"
        done
    done
done

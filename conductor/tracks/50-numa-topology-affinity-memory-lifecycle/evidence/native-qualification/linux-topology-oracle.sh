#!/usr/bin/env bash
set -euo pipefail
awk '/^Cpus_allowed_list:/ { print "allowed\t" $2 }' /proc/self/status
for node in /sys/devices/system/node/node[0-9]*; do
    id=${node##*node}
    memory=$(awk '/MemTotal:/ {printf "%.0f", $4 * 1024}' "$node/meminfo")
    printf 'node\t%s\t%s\t%s\n' "$id" "$memory" "$(cat "$node/cpulist")"
done
for core in /sys/devices/system/cpu/cpu[0-9]*/topology/thread_siblings_list; do
    printf 'core\t%s\n' "$(cat "$core")"
done | sort -u
for cache in /sys/devices/system/cpu/cpu[0-9]*/cache/index[0-9]*; do
    size=$(cat "$cache/size")
    case "$size" in
        *K) bytes=$(("${size%K}" * 1024));;
        *M) bytes=$(("${size%M}" * 1024 * 1024));;
        *) exit 2;;
    esac
    printf 'cache\t%s\t%s\t%s\t%s\t%s\n' \
        "$(cat "$cache/level")" "$(cat "$cache/type")" "$bytes" \
        "$(cat "$cache/coherency_line_size")" "$(cat "$cache/shared_cpu_list")"
done | sort -u

#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
unset KAIRO_NUMA_NODES
probe=$(realpath -- "$1")
mode=$2
if [[ "$mode" != paired && "$mode" != calibrate ]]; then
    printf 'Usage: %s /path/to/scheduler-locality paired|calibrate\n' "$0" >&2
    exit 2
fi

date -u +%FT%TZ
uname -smr
lscpu | sed -nE '/^(Model name|Hypervisor vendor|Virtualization type|NUMA node)/p'
for mask in online has_cpu has_memory; do
    printf '%s=' "$mask"
    cat "/sys/devices/system/node/$mask"
done
printf 'numa_balancing='
if [[ -r /proc/sys/kernel/numa_balancing ]]; then
    cat /proc/sys/kernel/numa_balancing
else
    printf 'unavailable\n'
fi
sha256sum "$probe" | awk '{print "scheduler-locality sha256=" $1}'

if [[ "$mode" == calibrate ]]; then
    for rejected in check local remote; do
        result=0
        "$probe" "$rejected" || result=$?
        printf '%s exit=%s\n' "$rejected" "$result"
        test "$result" -eq 77
    done
    timeout --kill-after=3 60 "$probe" calibrate
    printf 'calibrate exit=0\n'
    exit
fi

"$probe" check
for ((pair=1; pair<=10; pair++)); do
    if ((pair % 2)); then
        arms=(local remote)
    else
        arms=(remote local)
    fi
    for arm in "${arms[@]}"; do
        result=$(timeout --kill-after=3 60 "$probe" "$arm")
        printf 'pair=%s %s\n' "$pair" "$result"
    done
done
printf 'paired exit=0\n'

#!/usr/bin/env bash
set -euo pipefail

capture() {
    local group=$1
    printf '%s\n' "$BASHPID" > "$group/cgroup.procs"
    test "$(nproc)" -eq 4
    grep -E '^Cpus_allowed_list:[[:space:]]+4-7$' "/proc/$BASHPID/status" > samples/affinity.txt
    cat "$group/cpuset.cpus.partition" "$group/cpuset.cpus.effective" \
        "$group/cpuset.cpus.exclusive.effective" > samples/partition.txt
    local seed repeats count cost pair arm name
    seed=$(jq -er .seed protocol.json)
    repeats=$(jq -er .timed_repeats_per_process protocol.json)
    count=$(jq -er .triplets_per_cost protocol.json)
    mapfile -t costs < <(jq -er '.work_mixes[]' protocol.json)
    mapfile -t orders < <(jq -er '.order[] | join(" ")' protocol.json)
    test "${#orders[@]}" -eq 6
    read_cpus > samples/cpus-before.txt
    for cost in "${costs[@]}"; do
        for ((pair=0; pair<count; pair++)); do
            read -r -a arms <<< "${orders[pair % 6]}"
            for arm in "${arms[@]}"; do
                name="samples/$cost-$pair-$arm"
                /usr/bin/time -o "$name.rss" -f '%M %x' "./frontier-$arm" \
                    --phold --seed "$seed" --repetitions "$repeats" --event-work "$cost" \
                    > "$name.json" 2> "$name.stderr"
            done
        done
    done
    read_cpus > samples/cpus-after.txt
    cat "$group/cpu.stat" > samples/cpu-accounting.txt
    test "$(cat "$group/cpuset.cpus.partition")" = root
    test "$(cat "$group/cpuset.cpus.exclusive.effective")" = 4-7
}

read_cpus() {
    awk '$1 ~ /^cpu[4-7]$/ {print}' /proc/stat
}

if [[ ${1:-} == --capture ]]; then
    [[ $2 == /sys/fs/cgroup/kairos-track111-run.* ]]
    capture "$2"
    exit
fi

sha256sum --check linux-binaries.sha256
mkdir samples
exec 9>/run/lock/kairos-track111-perf.lock
flock -n 9
root=/sys/fs/cgroup
system_cpus=$(cat "$root/system.slice/cpuset.cpus.effective")
test "$system_cpus" = 0-7
{
    date -u '+start=%Y-%m-%dT%H:%M:%SZ'
    uname -smr
    lscpu | grep -E '^(Model name|Thread\(s\) per core|Core\(s\) per socket|CPU max MHz):'
    printf '%s\n' "system.slice before=$system_cpus"
} > samples/host.txt
group=$(mktemp -d "$root/kairos-track111-run.XXXXXX")
release() {
    rmdir "$group"
    test "$(cat "$root/system.slice/cpuset.cpus.effective")" = "$system_cpus"
    printf '%s\n' "system.slice restored=$system_cpus" > samples/release.txt
}
trap release EXIT
cat "$root/cpuset.mems.effective" > "$group/cpuset.mems"
printf '%s\n' 4-7 > "$group/cpuset.cpus"
printf '%s\n' root > "$group/cpuset.cpus.partition"
test "$(cat "$group/cpuset.cpus.partition")" = root
test "$(cat "$group/cpuset.cpus.exclusive.effective")" = 4-7
test "$(cat "$root/system.slice/cpuset.cpus.effective")" = 0-3
printf '%s\n' 'system.slice during=0-3' >> samples/host.txt
timeout --kill-after=5 180 bash "$0" --capture "$group"
date -u '+end=%Y-%m-%dT%H:%M:%SZ' >> samples/host.txt

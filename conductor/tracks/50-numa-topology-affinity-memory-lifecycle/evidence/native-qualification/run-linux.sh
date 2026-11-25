#!/usr/bin/env bash
set -euo pipefail
cd "$1"
sha256sum -c linux-executables.sha256
{
    uname -smr
    lscpu | grep -E '^(Model name|Thread\(s\) per core|NUMA node\(s\)):'
    printf 'hwloc '
    qualified/usr/bin/lstopo-no-graphics --version
    taskset -c 4-7 sh -c 'grep "^Cpus_allowed_list:" /proc/self/status'
} > linux-host.txt
./hwloc-readback-linux "$PWD/qualified/usr/bin/lstopo-no-graphics" > hwloc-linux.tsv
bash linux-topology-oracle.sh > sysfs-linux.tsv
env -u KAIRO_NUMA_NODES ./linux-numa-tests --nocapture --test-threads=1 > linux-affinity.log
timeout --kill-after=3 120 taskset -c 4-7 bash run-allocation.sh ./scheduler-contention-linux > allocation-linux.csv
test "$(wc -l < allocation-linux.csv)" -eq 320

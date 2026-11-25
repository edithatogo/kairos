#!/usr/bin/env bash
# Use a disposable VM and a private loopback-only network and mount namespace.
set -euo pipefail

if [[ ${1:-} == mounted ]]; then
    shift
    root=$1
    test_user=$2
    library_path=$3
    h5dump=$4
    mode=$5
    raw="$root/raw"
    mountpoint="$root/mount"
    export CEPH_CONF="$root/ceph.conf"
    ip link set lo up
    fsid=$(cat /proc/sys/kernel/random/uuid)
    mkdir "$mountpoint" "$root/mon-a" "$root/mgr-a" "$root/mds-a"
    cat > "$CEPH_CONF" <<CONF
[global]
fsid = $fsid
monmap = $root/monmap
mon host = 127.0.0.1:6789
ms bind msgr2 = false
auth cluster required = none
auth service required = none
auth client required = none
osd pool default size = 1
osd pool default min size = 1
mon allow pool size one = true
osd memory target = 671088640
bluestore block size = 2147483648
run dir = $root
log to file = false
log to stderr = true
[mon.a]
mon data = $root/mon-a
[mgr.a]
mgr data = $root/mgr-a
[mds.a]
mds data = $root/mds-a
CONF
    processes=()
    cleanup() {
        if mountpoint -q "$mountpoint"; then
            cd "$root"
            umount "$mountpoint"
        fi
        if (( ${#processes[@]} > 0 )); then
            kill "${processes[@]}" 2>/dev/null || true
        fi
    }
    trap cleanup EXIT
    trap 'exit 143' TERM INT
    monmaptool --create --fsid "$fsid" --add a 127.0.0.1:6789 "$root/monmap"
    ceph-authtool --create-keyring "$root/mon.keyring" --gen-key -n mon. --cap mon 'allow *' > "$root/key-setup.log"
    ceph-mon --mkfs -i a --monmap "$root/monmap" --keyring "$root/mon.keyring"
    ceph-mon -f -i a > "$raw/monitor.log" 2>&1 &
    processes+=("$!")
    timeout 30s ceph -s
    ceph-mgr -f -i a > "$raw/manager.log" 2>&1 &
    processes+=("$!")

    for expected_id in 0 1; do
        osd_uuid=$(cat /proc/sys/kernel/random/uuid)
        osd_id=$(timeout 20s ceph osd create "$osd_uuid")
        test "$osd_id" = "$expected_id"
        mkdir "$root/osd-$osd_id"
        ceph-osd --mkfs -i "$osd_id" --osd-uuid "$osd_uuid" --osd-data "$root/osd-$osd_id" \
            > "$raw/osd-$osd_id-init.log" 2>&1
        test -f "$root/osd-$osd_id/block"
        ceph-osd -f -i "$osd_id" --osd-data "$root/osd-$osd_id" > "$raw/osd-$osd_id.log" 2>&1 &
        osd_pid=$!
        processes+=("$osd_pid")
        osd_ready='any(.osds[]; .osd == $id and .up == 1 and .in == 1)'
        osd_state="$raw/osd-$osd_id-state.json"
        for _ in {1..45}; do
            if timeout 3s ceph osd dump -f json > "$osd_state" \
                && jq -e --argjson id "$osd_id" "$osd_ready" "$osd_state" >/dev/null; then
                break
            fi
            kill -0 "$osd_pid"
            sleep 1
        done
        jq -e --argjson id "$osd_id" "$osd_ready" "$osd_state"
    done
    timeout 5s ceph osd stat -f json | tee "$raw/osd-status.json" \
        | jq -e '.num_up_osds==2 and .num_in_osds==2'
    timeout 15s ceph osd pool create track51_data 32
    timeout 15s ceph osd pool create track51_meta 8
    timeout 15s ceph fs new track51 track51_meta track51_data
    ceph-mds -f -i a > "$raw/metadata.log" 2>&1 &
    processes+=("$!")
    active_mds='(.filesystem // .).mdsmap.info | any(.rank == 0 and .state == "up:active")'
    for _ in {1..45}; do
        if timeout 3s ceph fs get track51 -f json > "$raw/fs-state.json" \
            && jq -e "$active_mds" "$raw/fs-state.json" >/dev/null; then
            break
        fi
        sleep 1
    done
    jq -e "$active_mds" "$raw/fs-state.json"
    timeout 45s mount -t ceph 127.0.0.1:6789:/ "$mountpoint" \
        -o "conf=$CEPH_CONF,fs=track51,ms_mode=legacy"
    findmnt -T "$mountpoint" -o SOURCE,FSTYPE,TARGET,OPTIONS | tee "$raw/mount.txt"
    test "$(findmnt -T "$mountpoint" -no FSTYPE)" = ceph
    timeout 10s ceph osd tree > "$raw/osd-tree.txt"
    case_dir="$mountpoint/qualification"
    install -d -m 0750 -o "$test_user" "$case_dir"
    # Each 64 KiB stripe unit is an object, so a 2 MiB stream spans dozens of objects.
    setfattr -n ceph.dir.layout -v 'stripe_unit=65536 stripe_count=2 object_size=65536 pool=track51_data' "$case_dir"
    runuser -u "$test_user" -- bash -euo pipefail -c '
        mkdir "$1/preflight"
        cd "$1/preflight"
        printf original > staging
        sync staging
        ln staging published
        printf replacement > updated
        mv -T updated published
        sync .
        test "$(cat published)" = replacement
    ' bash "$case_dir" | tee "$raw/publication-preflight.log"
    printf 'Kernel mount, link, replacement and directory sync passed.\n' | tee -a "$raw/publication-preflight.log"
    touch "$root/ready"

    # Cargo builds in the original network namespace after publication passes.
    while [[ ! -f "$root/compiled" ]]; do
        sleep 1
    done
    mapfile -t binaries < "$root/binaries"
    bench=${binaries[0]}
    world=${binaries[1]}
    cd "$case_dir"
    runuser -u "$test_user" -- env TMPDIR="$case_dir" LD_LIBRARY_PATH="$library_path" \
        "$world" --exact hdf5_restores_the_ecs_world_in_a_fresh_process --nocapture \
        | tee "$raw/restart.log"
    grep -F 'test result: ok. 1 passed; 0 failed; 0 ignored' "$raw/restart.log"

    # Retain the two physical generations and the records compared by the test.
    runuser -u "$test_user" -- mkdir "$case_dir/engine"
    world_path="$case_dir/engine/world"
    for role in reference write recut resume; do
        runuser -u "$test_user" -- env LD_LIBRARY_PATH="$library_path" \
            KAIRO_WORLD_RESTART_ROLE="$role" KAIRO_WORLD_RESTART_PATH="$world_path" \
            KAIRO_WORLD_RESTART_FORMAT=hdf5 \
            "$world" --exact world_checkpoint_process --nocapture | tee "$raw/world-$role.log"
        if [[ "$role" == write || "$role" == recut ]]; then
            install -m 0644 "$world_path" "$raw/world-$role.h5"
            LD_LIBRARY_PATH="$library_path" "$h5dump" -H "$world_path" > "$raw/world-$role-layout.txt"
        fi
    done
    sed -n 's/^WORLD_RESULT=//p' "$raw/world-reference.log" > "$raw/world-reference.jsonl"
    sed -n 's/^WORLD_RESULT=//p' "$raw/world-recut.log" "$raw/world-resume.log" > "$raw/world-restored.jsonl"
    test "$(wc -l < "$raw/world-reference.jsonl")" -eq 2
    cmp "$raw/world-reference.jsonl" "$raw/world-restored.jsonl"
    (cd "$raw" && sha256sum ./world-write.h5 ./world-recut.h5) > "$raw/checkpoint-digests.log"
    if [[ "$mode" == restart-only ]]; then
        cd "$root"
        umount "$mountpoint"
        exit 0
    fi

    runuser -u "$test_user" -- env TMPDIR="$case_dir" \
        KAIROS_IPC_BENCHMARK_CAPTURE="$case_dir/retained" CRITERION_HOME="$raw/criterion" \
        "$bench" --bench contiguous_ipc_publication --noplot --sample-size 10 \
        --warm-up-time 1 --measurement-time 3 2>&1 | tee "$raw/benchmark.log"
    runuser -u "$test_user" -- mpirun --oversubscribe -np 2 "$root/mpi-copy" \
        "$case_dir/retained/part-0.arrows" "$case_dir/mpi-copy.arrows" | tee "$raw/mpi.log"
    cmp "$case_dir/retained/part-0.arrows" "$case_dir/mpi-copy.arrows"

    mkdir "$raw/streams"
    timeout 20s rados -p track51_data ls | sort > "$raw/data-objects.txt"
    for part in "$case_dir"/retained/*.arrows "$case_dir/mpi-copy.arrows"; do
        name=$(basename "$part")
        stat -c '%n %s bytes, preferred I/O block %o bytes' "$part" | tee -a "$raw/file-layout.log"
        getfattr --absolute-names -n ceph.file.layout "$part" | tee -a "$raw/file-layout.log"
        test "$(getfattr --only-values -n ceph.file.layout.stripe_count "$part")" = 2
        test "$(getfattr --only-values -n ceph.file.layout.stripe_unit "$part")" = 65536
        prefix=$(printf '%x.' "$(stat -c %i "$part")")
        object_map="$raw/$name-placement.tsv"
        while IFS= read -r object; do
            timeout 6s ceph osd map track51_data "$object" -f json \
                | jq -er '[.objname, (.acting[0] | tostring)] | @tsv' >> "$object_map"
        done < <(awk -v prefix="$prefix" 'index($0,prefix)==1' "$raw/data-objects.txt")
        test "$(cut -f 2 "$object_map" | sort -nu | paste -sd, -)" = 0,1
        install -m 0644 "$part" "$raw/streams/$name"
    done
    (cd "$raw/streams" && sha256sum ./*.arrows) > "$raw/file-digests.log"

    cd "$root"
    umount "$mountpoint"
    exit 0
fi

root="$RUNNER_TEMP/track51"
raw="$root/raw"
here=$(cd "$(dirname "$0")" && pwd)
mode=${1:-full}
[[ "$mode" == full || "$mode" == restart-only ]]
{
    git rev-parse HEAD
    uname -srmo
    cat /etc/os-release
    rustc +1.99.1 -Vv
    cargo +1.99.1 -V
    cc --version
    lscpu
    free -b
    ceph --version
    mpirun --version
    "$KAIROS_H5DUMP" --version
} > "$raw/runtime.log"
if [[ "$mode" == full ]]; then
    mpicc -std=c11 -Wall -Wextra -Werror "$here/mpi-copy.c" -o "$root/mpi-copy"
    mkdir "$raw/criterion"
fi
sudo timeout 1700s unshare --net --mount --propagation private \
    bash "$here/run-cephfs.sh" mounted "$root" "$(id -un)" "$LD_LIBRARY_PATH" "$KAIROS_H5DUMP" "$mode" \
    > "$raw/cephfs.log" 2>&1 &
mount_job=$!
trap 'sudo kill -TERM "$mount_job" 2>/dev/null || true' EXIT
for _ in {1..180}; do
    if [[ -f "$root/ready" ]]; then
        break
    fi
    if ! kill -0 "$mount_job" 2>/dev/null; then
        break
    fi
    sleep 1
done
if [[ ! -f "$root/ready" ]]; then
    tail -50 "$raw/cephfs.log"
    exit 1
fi

bench=
if [[ "$mode" == full ]]; then
    cargo +1.99.1 bench --locked -p kairo-ecs-arrow --features parallel-io,polars-native \
        --bench run_output --no-run --message-format=json > "$raw/bench-build.jsonl"
    bench=$(jq -r 'select(.target.name=="run_output" and .executable!=null) | .executable' "$raw/bench-build.jsonl")
    test -x "$bench"
fi
cargo +1.99.1 test --locked -p kairo-ecs-bench --features engine-checkpoint,hdf5 \
    --test world_checkpoint_restart --no-run --message-format=json > "$raw/restart-build.jsonl"
world=$(jq -r 'select(.target.name=="world_checkpoint_restart" and .executable!=null) | .executable' "$raw/restart-build.jsonl")
test -x "$world"
printf '%s\n%s\n' "$bench" "$world" > "$root/binaries"
touch "$root/compiled"
if wait "$mount_job"; then
    trap - EXIT
    cat "$raw/cephfs.log"
else
    status=$?
    tail -80 "$raw/cephfs.log"
    exit "$status"
fi

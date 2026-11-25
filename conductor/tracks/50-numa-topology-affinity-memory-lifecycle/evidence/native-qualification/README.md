# Track 50 native qualification

## Topology and affinity

The explicit Rust topology query invoked hwloc 2.7.0 on physical Linux
x86-64. Its 29 records matched independent sysfs readback exactly after
normalizing CPU ranges. The records cover one CPU mask, eight physical
core groups, 19 caches with sizes and sharing sets, and one NUMA domain
with 33,391,505,408 bytes. Two Linux integration tests passed. The
affinity case changed 0–7 to 0 and restored 0–7 on the calling thread.

The same Rust query ran with hwloc 2.16.0 on macOS ARM64 and returned
one node, 10 core groups and 23 caches. A parent process override using
`HWLOC_SYNTHETIC` produced byte-identical native readback through the
clean child environment. Unit tests cover multiword CPU masks, memory-only
nodes, omitted memory size, malformed XML and executable failure.
[Linux probe](hwloc-linux.tsv) · [Independent sysfs records](sysfs-linux.tsv) ·
[Linux affinity receipt](linux-affinity.log) · [macOS readback](hwloc-macos.tsv).

The optional discovery runs an explicitly selected hwloc executable and
reads its XML topology export. The existing `NumaTopology` supplies the
kernel and synthetic profiles. The detailed topology reports OS CPU IDs
and the mask returned by hwloc. A calling thread can have a more
restrictive affinity mask. [hwloc command-line reference](https://hwloc.readthedocs.io/en/v2.15/doxygen/html/tools.html).

## Scheduler allocation

Each timed group runs four independent production schedulers with the
same schedule, cancellation and dispatch checks. Two peaks use 256 and
2,048 queued events. Each process runs eight warm-up epochs and 256
timed epochs per scheduler. One eighth of events cancel. The probes
verify ticks, insertion order, event indices and generations, kind and
final counters. Ten fresh processes cover each serial or parallel
comparison at each peak with alternating run order.

The reservation budget is zero observed Rust system allocator calls in
the timed scheduler region and zero requested scheduler bytes remaining
after drop. All 160 reserved workers per host met both limits. Both host
runs observed occasional calls in the growing arm, up to two per
affected sample. The reserved path made two calls before timing in each
sample. It reserves extra cancellation storage in return for the measured
reuse. Before drop the reserved macOS scheduler held 37,896 or 303,112
requested bytes, and the Linux scheduler held 37,904 or 303,120 bytes at
the two peaks.

The counting allocator uses per-thread counters. Serial timing sums four
worker durations and the parallel timing spans their overlapping region.
Linux measured the named four CPUs under `taskset` on the shared physical
host. The samples report requested storage and wall times for these
workloads, with construction, warm-up and teardown outside the timed
region. [Protocol](allocation-protocol.json) · [macOS samples](allocation-macos.csv) ·
[Linux samples](allocation-linux.csv) · [Summaries and source hashes](receipt.json).

The macOS allocation comparison built both constructors in the same
binary at `c44a17b`. Linux built both at `6569d406` with `numa`.
Both include the same scheduler reservation and dispatch code. The
initial Mac calibration identified parking allocations outside the scheduler.
The measured capture used a bounded, allocation-free start gate. The
saved CSV records the trial number ahead of the probe's output columns.

## Native C Data lifetime

The locked `kairo-ecs-ffi` `native-data` library ran from a scratch
installation on macOS ARM64. A C11 consumer advanced the Rust engine and
imported its first two-row event-log stream batch. It released the engine
and stream, then read the aligned tick buffer containing 10 and 20. The
array's original release callback ran once.

The CPython 3.15.2 consumer imported the exact same exported buffer
address through PyArrow 26.0.1. The buffer survived dropping the parent
record batch and column. Releasing the last view forwarded the original
native release callback once. The qualification covers the C Data handoff
of the exported buffer. The producer creates its Arrow stream from
materialized engine event records. The existing three Rust C ABI tests
and ten Polars/C Data ownership tests also passed.
[Consumer receipt](ffi-result.txt) · [Library and consumer hashes](ffi-artifacts.sha256).

## Runtime routes for the remaining placement check

The physical NAS, the Apple Container x86-64 guest and the Google Colab
CLI CPU session each showed online, CPU-bearing and memory-bearing node
0. The Colab probe used the provider's Python 3.13.18 solely to read the
kernel masks and then released its isolated CPU session. All repository
analysis and consumer code used the verified local CPython 3.15.2 runtime.
[Apple Container readback](apple-container-numa.txt) · [Colab readback](colab-numa.json).

Cross-node page placement and the matched local and remote performance
comparison require at least two live memory-bearing NUMA nodes. That
acceptance gate remains open. [Track plan](../../plan.md).

## Reproduce

Use Rust and Cargo 1.99.1, an isolated output directory and the verified
repo-local Python 3.15.2 interpreter. Verify the retained files with
`shasum -a 256 -c SHA256SUMS` from this evidence directory.

From the repository root, compare both retained allocation captures and
replay the topology comparison.

```sh
evidence=conductor/tracks/50-numa-topology-affinity-memory-lifecycle/evidence/native-qualification
.venv/bin/python -I scripts/python_runtime.py
for platform in macos linux; do
    .venv/bin/python -I "$evidence/check-allocation.py" \
        "$evidence/allocation-protocol.json" "$evidence/allocation-$platform.csv"
done
.venv/bin/python -I "$evidence/check-topology.py" \
    "$evidence/hwloc-linux.tsv" "$evidence/sysfs-linux.tsv"
```

For the Linux build, extract the implementation source under
`/session/linux-source` in the pinned Linux x86-64 Rust container.
Place the evidence source files in `/session` with a resolved Cargo
cache, then run [build-linux.sh](build-linux.sh). Move the three
executables, their checksums and the capture scripts to a scratch
directory on the matching Linux host. Extract the hash-verified
Ubuntu hwloc package recorded in [receipt.json](receipt.json) at
`qualified/` in that directory and run `bash run-linux.sh "$PWD"`.
The executable receipts distinguish the workloads built in the container
from the measurements executed on the native runner.

The C Data consumer uses the native-data dylib selected from the actual
Cargo JSON build output. From the repository root use the pinned Rust
toolchain, the installed PyArrow profile and a fresh absolute
`install_dir`.

```sh
compiler=$(rustup which --toolchain 1.99.1 rustc)
export PATH="$(dirname "$compiler"):$PATH"
cargo rustc --release --locked -p kairo-ecs-ffi --features native-data \
    --crate-type cdylib --message-format=json > "$scratch/build.jsonl"
library=$(jq -rs '[.[]|select(.reason=="compiler-artifact" and .target.name=="kairo_ecs_ffi")|.filenames[]|select(endswith(".dylib"))][-1]' "$scratch/build.jsonl")
bash "$evidence/run-ffi.sh" "$library" "$install_dir"
```

The script relocates the two scratch consumers and verifies their loader
traces against the copied library. The library bytes match the Cargo
artifact.

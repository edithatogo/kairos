# CAP-50.5 scheduler page placement

The same locked Rust 1.99.1 executable ran on physical Linux x86-64 and
in the Apple Container Linux guest. Each one-node calibration observed
37,886 resident 4 KiB pages across the scheduler's two reservations on
node 0. Page counts matched immediately after reservation, after warmup
and after timing. Each run verified 9,175,040 dispatches, 1,310,720
cancellations, an empty queue and restoration of its original CPU mask.
Both hosts rejected the two-node modes with exit 77.
[Physical Linux](native-linux.log) · [Apple Container](apple-container.log).

The high-memory Colab CPU profile exposed eight CPUs and one memory
node. The G4 and V6E1 attempts ended during provisioning.
[CPU readback](colab-highmem.json) · [Profile outcomes](receipt.json).

## Matched comparison

The probe calls the production scheduler and affinity API. Its
observer forwards the scheduler allocations to `System` and
first-touches their complete pages on the selected allocation CPU.
The probe marks those pages with `MADV_DONTDUMP` to identify the exact
mapping boundaries, then checks residency against
`/proc/self/numa_maps`. Allocator bookkeeping and partial boundary
pages are outside the readback.
[Linux 6.12 mapping counters](https://docs.kernel.org/6.12/filesystems/proc.html).

Both arms run on the same CPU. They first-touch on its node for local
samples and on a CPU from the second node for remote samples. Each
process warms up twice on the execution CPU, then times eight epochs
of 1,048,576 events. One eighth cancel. A change in buffer identity,
page placement or scheduler results fails the sample. The runner
alternates arm order over ten fresh process pairs. The scheduler's
hash layout can vary across processes.

## Reproduce

The [receipt](receipt.json) identifies the build, sources and runtime
commands. Verify the bundle from this directory with
`shasum -a 256 -c SHA256SUMS`. To rebuild, extract the recorded Git tree
under `/session/source` in the pinned Apple Container image, place
`build-linux.sh` and `scheduler-locality.rs` in `/session`, and run
`bash /session/build-linux.sh` with the locked Cargo cache available.

On the target Linux host run `bash run-linux.sh ./scheduler-locality paired`.
The first-touch comparison needs two online memory nodes with an allowed CPU
on each. Preserve the raw pair rows, host topology and executable hash.
Use `calibrate` on a one-node host to exercise placement and check
the fail-closed host selection. The retained samples qualify one-node
calibration. CAP-50.5 still needs the paired native measurements.

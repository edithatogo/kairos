# Parallel I/O Benchmark Evidence

Track 51 benchmark output is evidence only when it preserves the storage context needed to reproduce the write path.

Minimum raw-result fields:

- command line, feature flags, and input scenario or seed manifest;
- writer format, checkpoint manifest path, and restart command when applicable;
- filesystem type, mount, stripe count, stripe size, block size, and rank count;
- scheduler, partition or queue, job ID, node count, and MPI implementation;
- raw output path and `sha256:` checksum;
- expected threshold and observed throughput or restart parity result.

The [Track 51 CephFS run](../../conductor/archive/51-parallel-io-arrow-checkpoint-export/handoff.md#cephfs-qualification--25-november-2026)
records four streams, both OSDs and a two-rank MPI-I/O copy on one VM.
Local filesystem runs remain useful for regression testing.

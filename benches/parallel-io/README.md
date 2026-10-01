# Parallel I/O Benchmark Evidence

Track 51 benchmark output is evidence only when it preserves the storage context needed to reproduce the write path.

Minimum raw-result fields:

- command line, feature flags, and input scenario or seed manifest;
- writer format, checkpoint manifest path, and restart command when applicable;
- filesystem type, mount, stripe count, stripe size, block size, and rank count;
- scheduler, partition or queue, job ID, node count, and MPI implementation;
- raw output path and `sha256:` checksum;
- expected threshold and observed throughput or restart parity result.

Local filesystem runs are fallback evidence. They may guard the benchmark harness and checkpoint contracts, but they do not satisfy the Track 51 `parallel-filesystem-evidence` gate until a reviewed Lustre, GPFS, or MPI-I/O manifest is recorded.

# Private local MPI launch evidence — 2026-10-05

## Scope and result

Four unchanged consumer runs passed with the private Open MPI 5.0.11 / external PRRTE 5.0.0rc1 / PMIx 7.0.0rc1 stack: Rust 1.76.0 and 1.99.0, each with two and four localhost ranks. This is bounded local collective evidence; production dependency adoption and full Track49 acceptance remain held.

Base commit: `8a7435d1ecb18d3cdd5a09910cd7c08d326741bf`. Working directory: this isolated worktree. Artifact directory: `artifacts/track49-private-launch-20261005` (ignored, local retained evidence). No engine/model/RNG input or seed was exercised.

## Bound execution

Executed `python3 artifacts/track49-private-launch-20261005/run_local.py`; exit 0. Reviewed runner SHA256: `4dc458fcd24714a38f5a927b7fbc9b0d3005be79c382472339e72c4db0ef4ef3`. The qualified distributed reviewer cleared this exact amended runner before execution. The original unexecuted proposal is preserved as `run_local-proposed-before-cleanup.py`. Exception-safe process-group kill/reap records supervision errors before asserting success.

Provider, alias and consumer binary maps were checked before and after every case. Their SHA256 values are respectively `cf914f4a34599159782b3733b23e250329760ee63252a1c659da20d6915a954f`, `8358b1b1e8acb5c4fe52e4c409dddb591283a13bbe9823c09d9cb61315bf24eb`, and `e8ddf8860772c96e1870e3462e37218aa16642f1e85e4d02aba4a18fa5113cc5`. Unchanged builds and toolchain binding are documented in `private-consumer-build-evidence-20261005.md` (SHA256 `a81fe397b0c058156851e97449c28c0c8e4b1d202978096bb27472d8ae55d0ab`).

Each exact launcher argv, environment, PID, epoch start/end, terminal status and raw-log hashes is retained in `receipts.json` and per-case environment files. Launch argv used the private `mpirun --host localhost --oversubscribe -n N -x TRACE_DIR -x TRACE_BINARY` with the hashed rank wrapper. Explicit controls selected PRRTE loopback, disabled remote PMIx connections, required PID match, selected native PMIx security, and selected Open MPI `self,sm` data transports.

| Rust | Ranks | Exit | Seconds | Rank result |
|---|---:|---:|---:|---|
| 1.76.0 | 2 | 0 | 1.726 | expected all-reduce, all-to-all, broadcast |
| 1.76.0 | 4 | 0 | 0.775 | expected all-reduce, all-to-all, broadcast |
| 1.99.0 | 2 | 0 | 0.932 | expected all-reduce, all-to-all, broadcast |
| 1.99.0 | 4 | 0 | 0.968 | expected all-reduce, all-to-all, broadcast |


Every case observed a launcher loopback TCP listener; none of the sampled launcher listeners was non-loopback. No timeout or supervision error occurred. Every rank passed the fixed sum/all-to-all/broadcast oracle. Wrapper rank/PID markers correlate matching dyld lines to each consumer process. Every observed non-system rank image matched the bound provider map or consumer binary; private libmpi, private libpmix and hwloc were required. `results.json` retains per-rank image/log hashes. A final `ps` readback of all four launcher and twelve rank PIDs returned exit 1 with empty stdout, confirming those known processes were absent at readback.

## Limits and remaining gates

Open MPI TCP BTL source binds a wildcard address, so this experiment deliberately used shared-memory/self data transport. TCP data transport, multi-node networking and continuous/all-daemon confinement were not tested. Socket sampling covered launcher PIDs only. Matching dyld image lines inventory observed images, not complete process/plugin closure. Native PMIx TCP credentials contain UID/GID and are not cryptographic peer authentication. Security selection/readback does not prove hostile-peer rejection. RC support/security policy, native test suites, full platform matrix and production provisioning remain unaccepted.

This consumer does not implement external accounting, checkpoint/model/RNG parity, rollback, recovery, migration, live gRPC or authoritative wire goldens. Material wire-schema approval and joint API/schema/dependency/storage freeze remain open. No production manifests, parent pins, global status, remote fork or release gate changed.

## Artifact integrity

The retained `evidence-hashes.json` inventories raw files before this record.
- `receipts.json` SHA256 `cfe84a4616752d2820a0eb7f22c5748d3517c4fd5c50e7b2f7413426401dc9c0`.
- `results.json` SHA256 `36b0885aba2e25a03f2b66310e3028caed3c3d2fc98ed88a07da27959646aee7`.
- `process-closeout.json` SHA256 `0af56e3a781359105cf8bf2cf64ce9473032e4a75413b9d4b2805d6ce5528588`.
- `evidence-hashes.json` SHA256 `aa323e136011927f7fc19188ec2ebd11f43d6644c5dcbfe2bdbd1500c426205f`.

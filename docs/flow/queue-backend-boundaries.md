# Flow queue and backend boundaries

The current experimental Flow queue is a single-world CPU facade. Its examples
and local queue fixtures do not establish backend portability. This page records
the limits needed when migrating a caller or comparing future execution paths.

| Area | Current documented boundary |
| --- | --- |
| Flow queue | One resource unit per request. Multi-resource work requires staged claims; this API provides no atomic multi-resource claim or deadlock-free acquisition. |
| Priority | Priority is per claim. No aging or starvation guarantee is provided. |
| Continuation | Pause/continue retains one live runtime in memory. It is not a portable checkpoint codec or cross-process restore. |
| PDES | Single-world queue fixtures and independent CPU replication do not establish cross-LP shared-resource trace equivalence or behavior through zero-lookahead cycles. |
| MPI/gRPC | No Flow queue transport/backend compatibility is claimed. Track 35's contract consumes Track 34 message/time bounds; transport semantics do not imply Flow integration. |
| Metal | Metal queue execution is unsupported by this Flow facade; device parity is not demonstrated. Backend selection describes wgpu/CUDA modules as not configured; CPU fallback is a separate path, not evidence of a Metal run. |

The [Q5.1 conformance boundary](../../conductor/design/queue/q5.1-conformance-20261004.md)
explicitly separates CPU replication invariance from PDES cross-LP traces, Metal
queue execution and portable codecs. Strict-priority starvation and Track 22/25
compatibility and release holds remain.

## Ownership and evidence

- [Track 22's queue checkpoint handoff](../../conductor/design/queue/q0.3-snapshot-track22-handoff-20260930.md)
  says a future portable contract must cover generations and allocator state,
  component encodings, queue/admission ordering, leases/revisions, scheduler and
  work state, and RNG/seed continuation. Its current snapshot and CLI handoffs do
  not restore Flow state.

- [Track 34's handoff](../../conductor/tracks/34-pdes-parallel-execution/handoff.md)
  describes a conservative PDES scaffold, not a production scheduler. Its
  [risk register](../../conductor/tracks/34-pdes-parallel-execution/risk-register.md)
  identifies deadlock risk for zero-lookahead cycles.

- [Track 35's Track 34 handoff](../../conductor/tracks/35-distributed-simulation-mpi-grpc/handoff-from-track34.md)
  defines remote event timestamps at or after local time plus declared lookahead,
  and null messages as the CMB time-bound signal. Those contracts do not make the
  Flow resource queue distributed.

- [Track 32](../../conductor/tracks/32-gpu-compute-acceleration/plan.md) owns later
  Metal/device queue parity and its evidence gates. Q5.3 does not implement or
  qualify that backend.
- [GPU backend selection](../gpu-compute/backend-selection.md) says the wgpu
  family is planned to map to Vulkan, Metal and DX12, while the current wgpu and
  CUDA modules report `backend-not-configured`. No device execution or Flow queue
  parity follows from the planned mapping.

- The [experimental Flow API review](../design/api-reviews/flow-runtime-q0.1.md)
  classifies the DES and ABM Flow roots as experimental. Track 25 compatibility
  review and release qualification remain separate.

Do not read the planned transport or backend contracts as current runtime
support. A future compatibility claim needs its own source-bound API review,
backend implementation, and evidence for the claimed workload and platform.

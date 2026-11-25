# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Semantic identity | Same tape and ordered per-LP state, with `production_parity` and `production_protocol` on both executors. | Matched ring, PHOLD and held-out ordered state passed. On Rust 1.99.1, 19 scoped and 20 pooled cases passed. |
| Overhead | Thread identity across two horizons and a native stack sample of the original scoped runtime. | 64 distinct callback threads before the change and one with the pool. [Native worker stacks](evidence/profiling/README.md#thread-creation-and-synchronization). |
| Allocation traffic | Allocator counts for matched setup and dispatch, plus separate uninstrumented timings with balanced order. | [Direct handoff experiment](handoff.md#direct-event-handoff-experiment) and recorded adoption decision. |
| Performance | Raw paired wall-clock and process RSS from the matched ring on named hardware. | Five event costs and eight historical cases recorded on Apple M1 Max. |
| Multiround PHOLD | Seeded tape, exact per-LP result and counter parity, cold and ten paired runs per profile with separate setup and dispatch. | [24 matched scoped and pooled cases on Apple M1 Max](evidence/phold/README.md). |
| Layout and checkpoint costs | Exact shared component values, fresh-process peak RSS, isolated native snapshot, serialization and byte transfer. | [Raw layout, checkpoint and IPC samples](evidence/profiling/README.md#storage-and-checkpoint-boundaries). |
| Held-out models | Ordered FIFO and spatial checks with exact replay, intentional negative inputs, matched totals and external SimPy oracle. | [Queueing](evidence/queueing/README.md) and [spatial/DES](evidence/spatial/README.md) passed. |
| Native backends | Actual device parity with full-run cost and observed transport/recovery boundaries. | [Metal, CUDA and transport evidence](evidence/backends/README.md) for named configurations. |
| Regression | Dedicated comparable runners evaluate declared thresholds; noisy shared-runner smoke is not performance certification. | Pending dedicated host. Both [shared-host frontier comparisons](evidence/frontier-fold/README.md) met their local rule. |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.


# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| S01–S04 | Receiving solvers and independent game oracles | [Retirements, bounded receiving APIs and Kuhn/Leduc evidence](../../scientific-completion.md#s01s04-paignia-transfer) |
| S05–S08 | Independent vectors, statistical fixtures and analytical hazard timing | [Bounded implementations, measured oracle limits and versioned trajectories](../../scientific-completion.md#qualified-s05s10-scopes) |
| S09–S10 | Nonidentity inference, scalar/batch parity and installed host lifetime tests | ONNX CPU and affine held-out predictions pass. Rust owns the queue worlds used by both host checkers. [Scope and receipts](../../scientific-completion.md#qualified-s05s10-scopes) |
| S11–S14 | Actual hardware, distributed processes, external FMUs and native file readers | [Metal, CUDA, MPI/gRPC, FMI and native I/O qualification](../../scientific-completion.md#s11-apple-metal-and-cuda-model-qualification) |
| S15 staged allocator | Same-tick rewind, exact replay, and edited fork isolation through live allocator actions | `cargo test --locked -p kairo-ecs-debug --example staged_allocator_replay` and [reviewed execution](handoff.md#s15-staged-allocator-replay--26-november-2026) |
| S17 bounded staged dispatch | Inclusive tick, event budget, cancelled close, and same-tick movement lease release | [Native DES and ABM checks](handoff.md#s17-bounded-allocator-dispatch--26-november-2026) |
| S17 coordinated dispatch | Earliest live event in either queue, allocation first at equal ticks, same-tick lease release before the next model event, and backdated work rejected in either queue | [Movement and allocator cases](handoff.md#s17-coordinated-allocator-and-model-dispatch--26-november-2026) |
| S17 native behavior dispatch | Preview preserves live events and clocks. The ABM runner invokes the behavior once, the model closes the lease at the update tick, and later work stays scheduled. | [Native ABM case](handoff.md#s17-native-behavior-dispatch--26-november-2026) |
| S17 separate DES and ABM queues | Allocator and model queue precedence at equal ticks, colliding event IDs routed to their owners, same-tick reactions, and a backdated event rejected against an idle clock | [Three-queue execution and review](handoff.md#s17-separate-des-and-abm-queues--26-november-2026) |
| S17 coordinated occupancy | The endpoint is reached and all due work drains. The export censors the live lease while the allocator is idle, seals the cutoff on success, and preserves the later arrival and release. | [Movement observation case](handoff.md#s17-occupancy-at-a-model-observation-tick--26-november-2026) |
| S17 boundary wakeups | DES expiry and arrival events interleave with ABM updates. Same-tick expiry precedes receipt and lease return. A late departure retry clears the expired transfer's arrival. | [Boundary and receiver cases](handoff.md#s17-boundary-wakeups--26-november-2026) |
| S17 overdue boundary arrivals | A tick-5 DES wakeup delivers the two tick-3 transfers before the tick-4 transfer. ABM preserves departure order at the tie and the tick-7 arrival stays queued. | [Overdue arrival case](handoff.md#s17-overdue-boundary-arrivals--27-november-2026) |
| S17 boundary arrival horizon | A seeded departure reaches the largest tick. A departure one tick later leaves the reserved transfer, wakeup preview, capacity and RNG state intact. | [Arrival horizon case](handoff.md#s17-boundary-arrival-horizon--27-november-2026) |
| S15–S17 | Checkpointed dispatches, real processes and boundary/resource conservation | [Native cases and separate PyArrow readback](evidence/native-qualification-20261125.json) |
| Scientific claims | Measured residual/error criteria and held-out tests for each advertised scope | [Qualified cases and explicit unsupported boundaries](../../capabilities.md). The named fixtures do not qualify additional platforms, empirical models or consumer migrations. |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.


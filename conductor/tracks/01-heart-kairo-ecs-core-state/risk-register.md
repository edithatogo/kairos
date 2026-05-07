# Risk Register — 01 The Heart: kairo-ecs-core & kairo-ecs-state

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Track docs drift away from the real lane map or implementation slice | 3 | 4 | 12 | Keep `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md` aligned with `lanes.md`, `plan.md`, and the current crate surface | track-owner | A doc update changes scope but does not update the lane or validation artifacts |
| Scheduler determinism regresses across time, priority, or sequence ordering | 2 | 5 | 10 | Keep the ordering and cancellation tests in `kairo-ecs-core` and the conformance fixture expectations aligned | core-scheduler-agent | Dispatch order changes for identical inputs |
| Cancellation semantics leak stale or duplicate event handling | 2 | 4 | 8 | Keep regression coverage for unknown IDs, already-dispatched IDs, duplicate cancellation, and cancelled future events | core-scheduler-agent | A cancelled event can still affect the limit/dispatch outcome |
| State snapshots stop being deterministic for downstream consumers | 2 | 4 | 8 | Preserve sorted `WorldSnapshot` ordering by `(index, generation)` and keep snapshot tests in the state crate | ecs-agent | Snapshot iteration order depends on `HashSet` or other nondeterministic iteration |
| Generational component handling regresses on replacement or stale handles | 2 | 4 | 8 | Keep `ComponentStore` tests for same-entity replacement, stale generations, and superseding generations | ecs-agent | A stale handle can read, replace, or remove the current component for an index |
| Windows linker/toolchain setup may block full executable tests on some shells | 2 | 3 | 6 | Keep `cargo check --tests` in the matrix and rerun the full heart-crate suite when a proper MSVC linker environment is active | track-owner | The environment resolves `link.exe` to a non-MSVC binary or cannot find MSVC build tools |
| Facade/binding work starts before Track 02 and Track 12 are ready | 3 | 4 | 12 | Keep 01E limited to Rust facade readiness and do not start binding work until the lane prerequisites are accepted | core-scheduler-agent + ffi-agent | Any binding-path work begins without the Track 02 FFI contract and Track 12 fixture runner |
| Performance claims outpace the currently enforced local checks | 3 | 3 | 9 | Keep the check matrix honest about what can be run locally now and add benchmark gates when the shared harness is available | track-owner | A performance regression is claimed without a runnable benchmark gate |

# Native core beta scope

The frozen qualification profile is
[`native-core-v1`](../../packaging/native-beta-scope.json). Candidate
artifacts bind the profile, source commit, native target and file digests.
The first beta requires an accepted receipt for each platform and the
installed CareOps case in the [release gates](../release-gates.md).

| Platform | C package | Python package |
| --- | --- | --- |
| Linux x86-64 GNU (Ubuntu 24.04) | Shared object and C header | Installed `kairo-ecs` wheel, CPython 3.15.2 and matching shared object |
| macOS ARM64 (macOS 15) | Dynamic library and C header | Installed `kairo-ecs` wheel, CPython 3.15.2 and matching dynamic library |

The Rust SDK is an immutable source archive for Cargo 1.99.1 with the
provider lock and path-based dependencies. Its supported default-feature
surface is the following.

| Crate | Surface and contract |
| --- | --- |
| `kairo-ecs-types` | `SimTime`, `SimDuration`, `EntityId`, `EventId`, `EventKind`, `ScheduleRequest`, `DispatchedEvent` and `StepOutcome` use integer ticks and generational identities |
| `kairo-ecs-core` | `Scheduler` scheduling, cancellation, stepping, bounded runs and counters. Time, priority and insertion order determine dispatch. Callers submit events at the current or a future tick |
| `kairo-ecs-state` | `World` entity spawn, liveness and retirement plus typed `ComponentStore` insertion, lookup and removal |
| `kairo-ecs-rng` | `DeterministicStream` from a run seed or entity and purpose, integer draws and the existing deterministic stream contract |
| `kairo-ecs-des` | `DESContext`, FIFO `Resource`, and `StagedResourceRuntime` actor/resource creation, atomic requests, admission, lease release and forward occupancy windows |
| `kairo-ecs-abm` | `ABMContext`, `AgentBehavior` and `BehaviorSimulation` for native scheduling and behavior, with deterministic entity streams |

## C ABI v1 and Python

The C package installs [`kairo_ecs.h`](../../include/kairo_ecs.h). The v1
candidate supports these symbols.

- `kairo_ecs_ffi_version`, `kairo_ecs_engine_new`,
  `kairo_ecs_engine_free`, `kairo_ecs_engine_reset` and
  `kairo_ecs_engine_current_time`.
- `kairo_ecs_schedule_at`, `kairo_ecs_schedule_after`,
  `kairo_ecs_cancel_event`, `kairo_ecs_step`, `kairo_ecs_run_for`,
  `kairo_ecs_run_until`, `kairo_ecs_run_until_or_for` and `kairo_ecs_stats`.
- `kairo_ecs_last_error_message`, `kairo_ecs_telemetry_flush_ipc` and
  `kairo_ecs_buffer_free`.

The ABI uses unsigned 64-bit ticks and handles, signed 32-bit priority and
unsigned 32-bit event kind. Handle zero is a failure sentinel. Status-returning
calls use the header's numeric values. A stale or foreign event returns
`NOT_FOUND`, and freeing an unknown or already retired engine returns
`ALREADY_FREED`. Reset and free retire all pending event handles for that
engine. A past absolute tick or relative tick overflow returns zero without
enqueueing. The read-only time and statistics functions return zero or zeroed
statistics on an invalid handle.

Read the thread-local, borrowed error string after a failure. Copy it before
another error on that thread or before the thread ends. The v1 telemetry
function with the historical `_ipc` name returns a UTF-8 JSON counter snapshot.
The caller owns the allocation across engine reset or free and returns its
original pointer and length exactly once through `kairo_ecs_buffer_free`.

The Python surface is `NativeEngine`, `NativeBackendUnavailable` and
`NativeExecutionError`. The engine provides context management, `close`,
`reset`, `stats`, `backend`, `library_path`, `current_time_ticks`, scheduling,
cancellation, stepping and bounded runs. A process chooses an absolute verified
v1 library path explicitly. Engine objects own one Rust handle and cannot be
copied or serialized. Valid cancellation of an unknown event returns false.
Missing, incomplete or incompatible native libraries raise
`NativeBackendUnavailable`.

## Other capability owners

The matrix starts with these two platforms and this Python adapter. Track 63
owns other installed binding and target profiles. Track 70 owns Arrow/Polars
and Arrow C Data qualification. Track 114 retains preview movement and
optimistic occupancy, Track 48 retains optimistic execution and Track 109
retains the remaining backend scope. Tracks 110 and 114 retain experimental
scientific and game methods. The release gate separately records provenance,
review, performance and maintainer acceptance.

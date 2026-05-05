# FFI Guide

The stable C ABI is the universal compatibility layer for all language bindings.

## Architecture

```text
Rust core (kairo-ecs-core)
    ↓
FFI bridge (kairo-ecs-ffi) — #[no_mangle] extern "C" functions
    ↓                    ↓
UniFFI generated          Diplomat generated          Direct C consumers
(Python/R/...)           (C++/Kotlin/...)            (Go cgo, C# P/Invoke)
```

## ABI Version

Current ABI version: `1`

Check at runtime: `kairo_ecs_ffi_version() → uint32_t`

## Handle Model

All engine state is behind opaque integer handles. Host languages never receive raw Rust references.

```c
#include "kairo_ecs.h"

typedef uint64_t KairoEcsEngineHandle;
typedef uint64_t KairoEcsEventHandle;

KairoEcsEngineHandle kairo_ecs_engine_new(void);
KairoEcsStatusCode   kairo_ecs_engine_free(KairoEcsEngineHandle handle);
KairoEcsStatusCode   kairo_ecs_engine_reset(KairoEcsEngineHandle handle);
```

### Ownership rules
- Every `_new` function returns a handle the caller must `_free`.
- Every allocated buffer has a matching `kairo_ecs_buffer_free`.
- Double-free is detected and returns `KAIRO_ECS_ERR_ALREADY_FREED`.
- `KairoEcsEventHandle` values are returned by reference from scheduling calls and consumed by `kairo_ecs_cancel_event`; they become invalid after the event is dispatched or cancelled.

## Status Codes

```c
typedef enum KairoEcsStatusCode {
    KAIRO_ECS_OK               = 0,
    KAIRO_ECS_ERR_INVALID_ARGUMENT = 1,
    KAIRO_ECS_ERR_NOT_FOUND       = 2,
    KAIRO_ECS_ERR_ALREADY_FREED   = 3,
    KAIRO_ECS_ERR_PANIC           = 100,
} KairoEcsStatusCode;
```

Retrieve last error message: `kairo_ecs_last_error_message() → const char*`

The error string is thread-local and valid until the next FFI call on the same thread.

## Panic Boundary

No Rust panic crosses the FFI boundary. Every `extern "C"` function installs a panic hook via `std::panic::catch_unwind` that catches unwinding and converts it to `KAIRO_ECS_ERR_PANIC`.

## Complete ABI Surface

### Engine lifecycle
| Function | Signature | Purpose |
|---|---|---|
| `kairo_ecs_ffi_version` | `uint32_t → uint32_t` | Returns ABI version |
| `kairo_ecs_engine_new` | `void → KairoEcsEngineHandle` | Create engine handle |
| `kairo_ecs_engine_free` | `handle → KairoEcsStatusCode` | Destroy engine handle |
| `kairo_ecs_engine_reset` | `handle → KairoEcsStatusCode` | Reset engine state |
| `kairo_ecs_engine_current_time` | `handle → uint64_t` | Get current SimTime in ticks |

### Event scheduling
| Function | Signature | Purpose |
|---|---|---|
| `kairo_ecs_schedule_at` | `handle, at_ticks:uint64_t, priority:int32_t, kind:uint32_t → KairoEcsEventHandle` | Schedule event at absolute time |
| `kairo_ecs_schedule_after` | `handle, after_ticks:uint64_t, priority:int32_t, kind:uint32_t → KairoEcsEventHandle` | Schedule event after duration from now |
| `kairo_ecs_cancel_event` | `handle, event:KairoEcsEventHandle → KairoEcsStatusCode` | Cancel scheduled event by handle |

### Execution
| Function | Signature | Purpose |
|---|---|---|
| `kairo_ecs_step` | `handle → KairoEcsStatusCode` | Execute one event |
| `kairo_ecs_run_for` | `handle, max_events:uint64_t → KairoEcsStatusCode` | Run up to N events |
| `kairo_ecs_run_until` | `handle, time_limit_ticks:uint64_t → KairoEcsStatusCode` | Run until time limit in ticks |
| `kairo_ecs_run_until_or_for` | `handle, time_limit_ticks:uint64_t, max_events:uint64_t → KairoEcsStatusCode` | Run until time limit or max events, whichever comes first |

### Telemetry and stats
| Function | Signature | Purpose |
|---|---|---|
| `kairo_ecs_stats` | `handle → KairoEcsStats` | Get simulation statistics (returned by value) |
| `kairo_ecs_telemetry_flush_ipc` | `handle → KairoEcsBuffer` | Flush telemetry to IPC buffer |
| `kairo_ecs_last_error_message` | `void → const char*` | Get last error message |
| `kairo_ecs_buffer_free` | `buffer:KairoEcsBuffer → void` | Free allocated buffer |

## Data Structures

### KairoEcsStats
```c
typedef struct KairoEcsStats {
    uint64_t now_ticks;          // current simulation time in ticks
    uint64_t scheduled_events;   // total events scheduled since creation
    uint64_t dispatched_events;  // total events dispatched since creation
    uint64_t cancelled_events;   // total events cancelled since creation
    uint64_t pending_events;     // events currently in the queue
} KairoEcsStats;
```

Returned by value from `kairo_ecs_stats`. No allocation or free-step required.

### KairoEcsBuffer
```c
typedef struct KairoEcsBuffer {
    const uint8_t* data;  // pointer to allocated buffer
    size_t         len;   // length in bytes
} KairoEcsBuffer;
```

Returned from `kairo_ecs_telemetry_flush_ipc`. Must be freed with `kairo_ecs_buffer_free`.

## Buffer Semantics

- All returned buffers are callee-allocated, caller-freed.
- `kairo_ecs_last_error_message()` returns a thread-local error string valid until the next FFI call on the same thread.
- `KairoEcsStats` is a plain struct returned by value — no allocation, no free-step.
- `kairo_ecs_buffer_free` is safe to call with a null or zero-length buffer (no-op).

## Thread Safety

- `KairoEcsEngineHandle` is `Send` but not `Sync`.
- A single engine handle must be used from one thread at a time.
- Multiple engines can run concurrently on separate threads.
- The error message buffer is thread-local; each thread sees only its own last error.

## Writing a Binding

1. Load the native library (`dlopen`, `NativeLibrary.Load`, etc.)
2. Include `kairo_ecs.h` for C/C++ consumers, or define the C function signatures in your language's FFI system (P/Invoke, cgo, ccall, etc.)
3. Wrap handles in your language's resource management (SafeHandle, finalizer, with-disposal)
4. Map status codes to language-appropriate error types (exceptions, Result types, etc.)
5. Test with the conformance fixtures from `conformance/fixtures/`

## Testing

```bash
cargo test -p kairo-ecs-ffi
cargo test -p kairo-ecs-uniffi
cargo test -p kairo-ecs-diplomat
```

The FFI crate includes an in-source test that asserts the canonical header at `include/kairo_ecs.h` matches the generated surface. Any change to `lib.rs` must keep the header in sync.

## Conformance Fixtures

Test bindings against the JSON fixture files in `conformance/fixtures/`:

| Fixture | Purpose |
|---|---|
| `deterministic_ordering.json` | Event ordering determinism |
| `cancellation.json` | Event cancellation semantics |
| `rng_replay.json` | RNG replay across sessions |
| `vvuq_scenario_replay.json` | VVUQ scenario replay |

## Related Documents

- [FFI contract](../../conductor/contracts/ffi-contract.md)
- [Conformance contract](../../conductor/contracts/conformance-contract.md)
- [ADR 0002: Stable C ABI](../../docs/adr/0002-stable-c-abi-as-canonical-ffi-backstop.md)

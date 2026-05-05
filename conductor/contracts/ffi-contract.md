# FFI Contract

## Strategy

The stable C ABI is the universal compatibility layer. UniFFI and Diplomat are generated convenience layers, not the sole source of truth.

## Ownership

```text
KairoEcsEngineHandle is opaque.
Host languages never receive Rust references.
Every allocated buffer has a matching free function.
Every engine/resource handle has explicit close/free semantics.
Double-free must be detected or made harmless.
```

## Error handling

No panic crosses the FFI boundary.

```c
typedef enum KairoEcsStatusCode {
  KAIRO_ECS_OK = 0,
  KAIRO_ECS_ERR_INVALID_ARGUMENT = 1,
  KAIRO_ECS_ERR_NOT_FOUND = 2,
  KAIRO_ECS_ERR_ALREADY_FREED = 3,
  KAIRO_ECS_ERR_PANIC = 100,
} KairoEcsStatusCode;
```

Errors are retrieved through a per-engine or thread-local error buffer.

## Minimal ABI surface v1

```text
kairo_ecs_ffi_version
kairo_ecs_engine_new
kairo_ecs_engine_free
kairo_ecs_engine_reset
kairo_ecs_engine_current_time
kairo_ecs_schedule_at
kairo_ecs_schedule_after
kairo_ecs_cancel_event
kairo_ecs_step
kairo_ecs_run_for
kairo_ecs_run_until
kairo_ecs_stats
kairo_ecs_last_error_message
kairo_ecs_telemetry_flush_ipc
kairo_ecs_buffer_free
```

## Callback policy

Callbacks into host languages are allowed only in explicitly marked slow/prototyping APIs. Hot-loop modeling should use Rust-side systems, batch commands, or compiled plugins where possible.

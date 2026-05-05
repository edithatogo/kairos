#ifndef KAIRO_ECS_H
#define KAIRO_ECS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef uint64_t KairoEcsEngineHandle;
typedef uint64_t KairoEcsEventHandle;

typedef enum KairoEcsStatusCode {
  KAIRO_ECS_OK = 0,
  KAIRO_ECS_ERR_INVALID_ARGUMENT = 1,
  KAIRO_ECS_ERR_NOT_FOUND = 2,
  KAIRO_ECS_ERR_ALREADY_FREED = 3,
  KAIRO_ECS_ERR_PANIC = 100,
} KairoEcsStatusCode;

typedef struct KairoEcsBuffer {
  const uint8_t* data;
  size_t len;
} KairoEcsBuffer;

typedef struct KairoEcsStats {
  uint64_t now_ticks;
  uint64_t scheduled_events;
  uint64_t dispatched_events;
  uint64_t cancelled_events;
  uint64_t pending_events;
} KairoEcsStats;

uint32_t kairo_ecs_ffi_version(void);
KairoEcsEngineHandle kairo_ecs_engine_new(void);
KairoEcsStatusCode kairo_ecs_engine_free(KairoEcsEngineHandle handle);
KairoEcsStatusCode kairo_ecs_engine_reset(KairoEcsEngineHandle handle);
uint64_t kairo_ecs_engine_current_time(KairoEcsEngineHandle handle);
KairoEcsEventHandle kairo_ecs_schedule_at(KairoEcsEngineHandle handle, uint64_t at_ticks, int32_t priority, uint32_t kind);
KairoEcsEventHandle kairo_ecs_schedule_after(KairoEcsEngineHandle handle, uint64_t after_ticks, int32_t priority, uint32_t kind);
KairoEcsStatusCode kairo_ecs_cancel_event(KairoEcsEngineHandle handle, KairoEcsEventHandle event);
KairoEcsStatusCode kairo_ecs_step(KairoEcsEngineHandle handle);
KairoEcsStatusCode kairo_ecs_run_for(KairoEcsEngineHandle handle, uint64_t max_events);
KairoEcsStatusCode kairo_ecs_run_until(KairoEcsEngineHandle handle, uint64_t time_limit_ticks);
KairoEcsStatusCode kairo_ecs_run_until_or_for(KairoEcsEngineHandle handle, uint64_t time_limit_ticks, uint64_t max_events);
KairoEcsStats kairo_ecs_stats(KairoEcsEngineHandle handle);
const char* kairo_ecs_last_error_message(void);
KairoEcsBuffer kairo_ecs_telemetry_flush_ipc(KairoEcsEngineHandle handle);
void kairo_ecs_buffer_free(KairoEcsBuffer buffer);

#ifdef __cplusplus
}
#endif

#endif

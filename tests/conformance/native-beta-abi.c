#define _GNU_SOURCE
#define _DARWIN_C_SOURCE
#include "kairo_ecs.h"
#include <assert.h>
#include <dlfcn.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

_Static_assert(sizeof(KairoEcsEngineHandle) == 8, "engine handle width");
_Static_assert(sizeof(KairoEcsEventHandle) == 8, "event handle width");
_Static_assert(sizeof(KairoEcsStatusCode) == 4, "status width");
_Static_assert(KAIRO_ECS_ERR_PANIC == 100, "panic status");
_Static_assert(sizeof(KairoEcsStats) == 40, "statistics layout");
_Static_assert(offsetof(KairoEcsStats, pending_events) == 32, "pending offset");
_Static_assert(sizeof(KairoEcsBuffer) == 16, "buffer layout");
_Static_assert(offsetof(KairoEcsBuffer, len) == 8, "buffer length offset");

static void require_installed_library(const char *expected_path) {
  Dl_info location;
  char loaded[PATH_MAX];
  char expected[PATH_MAX];
  void *symbol = dlsym(RTLD_DEFAULT, "kairo_ecs_ffi_version");
  assert(symbol != NULL && dladdr(symbol, &location) != 0);
  assert(realpath(location.dli_fname, loaded) != NULL);
  assert(realpath(expected_path, expected) != NULL);
  assert(strcmp(loaded, expected) == 0);
}

int main(int argc, char **argv) {
  assert(argc == 2);
  require_installed_library(argv[1]);
  assert(kairo_ecs_ffi_version() == 1);

  KairoEcsEngineHandle a = kairo_ecs_engine_new();
  KairoEcsEngineHandle b = kairo_ecs_engine_new();
  assert(a != 0 && b != 0 && a != b);
  KairoEcsEventHandle foreign = kairo_ecs_schedule_at(b, 7, -1, 20);
  KairoEcsEventHandle low = kairo_ecs_schedule_at(a, 7, 0, 10);
  KairoEcsEventHandle first = kairo_ecs_schedule_at(a, 7, -1, 11);
  KairoEcsEventHandle second = kairo_ecs_schedule_at(a, 7, -1, 12);
  assert(foreign && low && first && second);
  assert(kairo_ecs_cancel_event(a, foreign) == KAIRO_ECS_ERR_NOT_FOUND);
  assert(kairo_ecs_stats(a).pending_events == 3);
  assert(kairo_ecs_step(a) == KAIRO_ECS_OK);
  assert(kairo_ecs_cancel_event(a, first) == KAIRO_ECS_ERR_NOT_FOUND);
  assert(kairo_ecs_run_until_or_for(a, 7, 1) == KAIRO_ECS_OK);
  assert(kairo_ecs_cancel_event(a, second) == KAIRO_ECS_ERR_NOT_FOUND);
  assert(kairo_ecs_cancel_event(a, low) == KAIRO_ECS_OK);
  assert(kairo_ecs_engine_current_time(a) == 7);
  assert(kairo_ecs_schedule_at(a, 6, 0, 1) == 0);
  assert(strstr(kairo_ecs_last_error_message(), "precedes") != NULL);
  assert(kairo_ecs_schedule_after(a, UINT64_MAX, 0, 1) == 0);
  assert(strstr(kairo_ecs_last_error_message(), "tick range") != NULL);

  KairoEcsStats stats = kairo_ecs_stats(a);
  assert(stats.now_ticks == 7 && stats.scheduled_events == 3);
  assert(stats.dispatched_events == 2 && stats.cancelled_events == 1);
  assert(stats.pending_events == 0);
  KairoEcsBuffer retained = kairo_ecs_telemetry_flush_ipc(a);
  assert(retained.data && retained.len);
  assert(kairo_ecs_engine_free(a) == KAIRO_ECS_OK);
  assert(kairo_ecs_engine_free(a) == KAIRO_ECS_ERR_ALREADY_FREED);
  assert(kairo_ecs_step(a) == KAIRO_ECS_ERR_NOT_FOUND);
  static const char expected[] =
      "{\"now_ticks\":7,\"scheduled_events\":3,\"dispatched_events\":2,"
      "\"cancelled_events\":1,\"pending_events\":0}";
  assert(retained.len == sizeof(expected) - 1);
  assert(memcmp(retained.data, expected, retained.len) == 0);
  kairo_ecs_buffer_free(retained);

  assert(kairo_ecs_engine_reset(b) == KAIRO_ECS_OK);
  assert(kairo_ecs_cancel_event(b, foreign) == KAIRO_ECS_ERR_NOT_FOUND);
  assert(kairo_ecs_schedule_at(b, UINT64_MAX, INT32_MIN, UINT32_MAX) != 0);
  assert(kairo_ecs_run_until(b, UINT64_MAX - 1) == KAIRO_ECS_OK);
  assert(kairo_ecs_stats(b).pending_events == 1);
  assert(kairo_ecs_run_for(b, 1) == KAIRO_ECS_OK);
  assert(kairo_ecs_engine_current_time(b) == UINT64_MAX);
  assert(kairo_ecs_schedule_after(b, 1, 0, 0) == 0);
  assert(kairo_ecs_engine_free(b) == KAIRO_ECS_OK);
  puts("Installed C ABI v1 passed dispatch, layout, error and lifetime checks.");
  return 0;
}

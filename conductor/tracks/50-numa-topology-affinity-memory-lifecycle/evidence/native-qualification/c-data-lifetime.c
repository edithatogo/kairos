#include "kairo_ecs.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

static void (*array_release)(KairoArrowArray *);
static unsigned releases;

static void observe_array_release(KairoArrowArray *array) {
    assert(array_release);
    array->release = array_release;
    array_release(array);
    assert(!array->release);
    array_release = NULL;
    ++releases;
}

unsigned track50_release_count(void) {
    return releases;
}

uintptr_t track50_export_batch(KairoArrowSchema *schema, KairoArrowArray *array) {
    KairoEcsEngineHandle engine = kairo_ecs_engine_new();
    assert(engine);
    assert(kairo_ecs_schedule_at(engine, 30, 0, 50));
    assert(kairo_ecs_schedule_at(engine, 10, 0, 50));
    assert(kairo_ecs_schedule_at(engine, 20, 0, 50));
    assert(kairo_ecs_run_for(engine, 3) == KAIRO_ECS_OK);

    KairoArrowArrayStream stream = {0};
    assert(kairo_ecs_export_event_log_arrow_stream(&engine, 2, &stream) == KAIRO_ECS_OK);
    assert(stream.get_schema(&stream, schema) == 0);
    assert(stream.get_next(&stream, array) == 0);
    assert(array->length == 2 && array->n_children == 14);
    assert(schema->n_children == array->n_children);
    assert(kairo_ecs_engine_free(engine) == KAIRO_ECS_OK);
    stream.release(&stream);
    assert(!stream.release);

    const uint64_t *ticks = NULL;
    for (int64_t index = 0; index < schema->n_children; ++index) {
        const KairoArrowSchema *field_schema = schema->children[index];
        if (strcmp(field_schema->name, "time_ticks_lo") == 0) {
            const KairoArrowArray *field_array = array->children[index];
            assert(strcmp(field_schema->format, "L") == 0);
            assert(field_array->offset == 0 && field_array->n_buffers == 2);
            ticks = field_array->buffers[1];
        }
    }
    assert(ticks && (uintptr_t)ticks % _Alignof(uint64_t) == 0);
    assert(ticks[0] == 10 && ticks[1] == 20);
    assert(array->release && !array_release);
    array_release = array->release;
    array->release = observe_array_release;
    return (uintptr_t)ticks;
}

#ifdef TRACK50_STANDALONE
int main(void) {
    KairoArrowSchema schema = {0};
    KairoArrowArray array = {0};
    assert(track50_export_batch(&schema, &array) != 0);
    assert(track50_release_count() == 0);
    schema.release(&schema);
    array.release(&array);
    assert(!schema.release && !array.release && track50_release_count() == 1);
    puts("C stream batch retained aligned ticks [10, 20] after both owners were released");
    puts("C root release callback ran once");
}
#endif

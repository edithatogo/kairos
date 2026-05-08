using Test
using KairoECS

@testset "event log smoke payload roundtrips" begin
    batch = EventLogBatch([
        EventLogRecord(
            run_id = "arrow-run",
            event_id = "event-1",
            entity_id = "entity-1",
            time_ticks = UInt128(123456789),
            priority = Int32(4),
            sequence = UInt64(9),
            event_kind = "custom:arrow",
            status = "dispatched",
            payload_ref = "payload://event-1",
        ),
    ])

    payload = to_smoke_bytes(batch)
    decoded = from_smoke_bytes(payload)

    @test decoded == batch
    @test startswith(String(payload), "stream=kairo_ecs.event_log.v1;schema_version=1")
    @test arrow_event_log_schema().fields[5].type == "FixedSizeBinary(16)"
end


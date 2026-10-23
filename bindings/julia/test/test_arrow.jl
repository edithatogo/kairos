using Test
using KairoECS

@testset "event log smoke payload roundtrips" begin
    batch = EventLogBatch([
        EventLogRecord(
            run_id = "arrow-run",
            event_id_index = UInt64(0x0000_0000_0000_0001),
            event_id_gen = UInt32(0),
            entity_id = (index = UInt64(0x0000_0000_0000_0002), gen = UInt32(0)),
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
    @test startswith(String(payload), "stream=kairo_ecs.event_log.v2;schema_version=2")
    @test arrow_event_log_schema().fields[7].type == "UInt64"
    @test arrow_event_log_schema().fields[8].type == "UInt64"
end

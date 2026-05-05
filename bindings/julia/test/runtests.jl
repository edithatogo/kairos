using Test
using KairoECS

@testset "KairoECS" begin
    @test version_string() == "0.1.0"
    @test self_check() == Dict(
        :package => "KairoECS",
        :version => "0.1.0",
        :status => "ok",
        :ffi_configured => "false",
    )
end

@testset "deterministic scheduler ordering" begin
    unordered = [
        EventLogRecord(
            run_id = "run-1",
            event_id = "event-c",
            time_ticks = UInt128(5),
            priority = Int32(1),
            sequence = UInt64(3),
            event_kind = "arrival",
        ),
        EventLogRecord(
            run_id = "run-1",
            event_id = "event-a",
            time_ticks = UInt128(1),
            priority = Int32(0),
            sequence = UInt64(2),
            event_kind = "start",
        ),
        EventLogRecord(
            run_id = "run-1",
            event_id = "event-b",
            time_ticks = UInt128(1),
            priority = Int32(0),
            sequence = UInt64(1),
            event_kind = "start",
        ),
    ]

    @test [event.event_id for event in ordered_events(unordered)] == ["event-b", "event-a", "event-c"]
    @test_throws ArgumentError ordered_events([
        EventLogRecord(
            run_id = "run-1",
            event_id = "event-invalid",
            time_ticks = UInt128(1),
            time_scale = "seconds",
            sequence = UInt64(1),
            event_kind = "start",
        ),
    ])
end

@testset "arrow event log schema facade" begin
    schema = arrow_event_log_schema()

    @test schema.stream == "kairo_ecs.event_log.v1"
    @test schema.schema_version == UInt16(1)
    @test [field.name for field in schema.fields] == [
        "schema_version",
        "run_id",
        "event_id",
        "entity_id",
        "time_ticks",
        "time_scale",
        "priority",
        "sequence",
        "event_kind",
        "status",
        "payload_ref",
    ]
    @test schema.fields[5].type == "FixedSizeBinary(16)"
    @test schema.fields[11].nullable
end

@testset "native ffi status" begin
    status = ffi_status()

    @test status.configured == false
    @test status.library === nothing
    @test occursin("not configured", status.reason)
    @test !is_ffi_configured()
end

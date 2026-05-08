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

@testset "arrow smoke roundtrip" begin
    records = [
        EventLogRecord(
            run_id = "run-1",
            event_id = "event-b",
            entity_id = "entity-1",
            time_ticks = UInt128(2)^80 + UInt128(42),
            priority = Int32(0),
            sequence = UInt64(2),
            event_kind = "custom:arrival",
            payload_ref = "payload://arrival",
        ),
        EventLogRecord(
            run_id = "run-1",
            event_id = "event-a",
            time_ticks = UInt128(7),
            priority = Int32(-1),
            sequence = UInt64(1),
            event_kind = "custom:start",
        ),
    ]

    batch = EventLogBatch(records)
    decoded = from_smoke_bytes(to_smoke_bytes(batch))

    @test decoded == EventLogBatch(ordered_events(records))
    @test [record.event_id for record in decoded.records] == ["event-a", "event-b"]
    @test decoded.records[2].time_ticks == UInt128(2)^80 + UInt128(42)
    @test decoded.records[2].payload_ref == "payload://arrival"
    @test_throws ArgumentError from_smoke_bytes(UInt8[])
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

@testset "conformance fixture bridge" begin
    fixtures = [
        ConformanceFixture(
            id = "scheduler_ordering_v1",
            status = "ready",
            kind = "ordering",
            source = "deterministic_ordering.json",
            consumers = ["01", "08"],
            assertions = ["order by time, priority, sequence"],
        ),
        ConformanceFixture(
            id = "des_resource_queue_v1",
            status = "planned",
            kind = "des",
            consumers = ["01", "08"],
            assertions = ["resource queue behavior is stable"],
        ),
        ConformanceFixture(
            id = "vvuq_scenario_replay_v1",
            status = "ready",
            kind = "vvuq",
            consumers = ["21", "22"],
            assertions = ["scenario manifest and seed manifest exist"],
        ),
    ]

    @test binding_fixture_ids(fixtures) == ["scheduler_ordering_v1", "des_resource_queue_v1"]
    @test ready_fixture_ids(fixtures) == ["scheduler_ordering_v1"]
    @test fixture_status(fixtures, "des_resource_queue_v1") == "planned"
    @test fixture_status(fixtures, "missing_fixture") === nothing

    report = conformance_report(fixtures)
    @test report.track_id == "08"
    @test report.ready == ["scheduler_ordering_v1"]
    @test report.planned == ["des_resource_queue_v1"]
    @test report.ready_count == 1
    @test report.planned_count == 1

    dict_fixtures = [
        Dict(
            "id" => "rng_reproducibility_v1",
            "status" => "ready",
            "kind" => "rng",
            "source" => "rng_replay.json",
            "consumers" => ["01", "08"],
            "assertions" => ["entity-derived RNG stays deterministic across bindings"],
        ),
    ]

    @test ready_fixture_ids(dict_fixtures) == ["rng_reproducibility_v1"]

    tuple_fixtures = [
        (
            id = "zero_delay_guard_v1",
            status = "ready",
            kind = "scheduler",
            source = "zero_delay_guard.json",
            consumers = ["01", "08"],
            assertions = ["zero-delay loops are rejected or guarded consistently"],
        ),
    ]

    @test binding_fixture_ids(tuple_fixtures) == ["zero_delay_guard_v1"]
end

include("test_arrow.jl")

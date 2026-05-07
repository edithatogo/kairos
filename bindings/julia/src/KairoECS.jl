module KairoECS

export EventLogRecord,
    arrow_event_log_schema,
    binding_fixture_ids,
    conformance_report,
    ConformanceFixture,
    fixture_status,
    ffi_status,
    is_ffi_configured,
    ordered_events,
    ready_fixture_ids,
    self_check,
    version_string

const VERSION_STRING = "0.1.0"
const EVENT_LOG_STREAM = "kairo_ecs.event_log.v1"
const EVENT_LOG_SCHEMA_VERSION = UInt16(1)
const JULIA_BINDING_TRACK_ID = "08"

version_string() = VERSION_STRING

"""
    EventLogRecord(; ...)

Pure-Julia representation of one scheduler event-log row. Ordering follows the
core contract: `(time_ticks, priority, sequence)`.
"""
Base.@kwdef struct EventLogRecord
    run_id::String
    event_id::String
    entity_id::Union{Nothing,String} = nothing
    time_ticks::UInt128
    time_scale::String = "ticks"
    priority::Int32 = Int32(0)
    sequence::UInt64
    event_kind::String
    status::String = "dispatched"
    payload_ref::Union{Nothing,String} = nothing
end

"""
    ConformanceFixture(; id, status, kind, consumers, source = nothing, assertions = String[])

Pure-Julia representation of a Track 12 fixture manifest entry. The binding
uses this as a fixture bridge until Track 12 wires a shared runner.
"""
Base.@kwdef struct ConformanceFixture
    id::String
    status::String
    kind::String
    consumers::Vector{String}
    source::Union{Nothing,String} = nothing
    assertions::Vector{String} = String[]
end

const _EVENT_LOG_FIELDS = (
    (name = "schema_version", type = "UInt16", nullable = false),
    (name = "run_id", type = "Utf8", nullable = false),
    (name = "event_id", type = "FixedSizeBinary(12)", nullable = false),
    (name = "entity_id", type = "FixedSizeBinary(12)", nullable = true),
    (name = "time_ticks", type = "FixedSizeBinary(16)", nullable = false),
    (name = "time_scale", type = "Utf8", nullable = false),
    (name = "priority", type = "Int32", nullable = false),
    (name = "sequence", type = "UInt64", nullable = false),
    (name = "event_kind", type = "Utf8", nullable = false),
    (name = "status", type = "Utf8", nullable = false),
    (name = "payload_ref", type = "Utf8", nullable = true),
)

function _validate_event(event::EventLogRecord)
    event.time_scale == "ticks" || throw(ArgumentError("time_scale must be ticks"))
    event.status in ("dispatched", "cancelled", "skipped", "error") ||
        throw(ArgumentError("unsupported event status: $(event.status)"))
    return event
end

"""
    ordered_events(events)

Return a new vector ordered deterministically by the scheduler key.
"""
function ordered_events(events)
    validated = [_validate_event(event) for event in events]
    return sort(validated; by = event -> (event.time_ticks, event.priority, event.sequence))
end

"""
    arrow_event_log_schema()

Return the Track 04 event-log schema facade without requiring Arrow.jl at load time.
"""
function arrow_event_log_schema()
    return (
        stream = EVENT_LOG_STREAM,
        schema_version = EVENT_LOG_SCHEMA_VERSION,
        fields = collect(_EVENT_LOG_FIELDS),
    )
end

function _string_vector(values)
    return String[string(value) for value in values]
end

function _fixture_from_record(record::ConformanceFixture)
    return record
end

function _fixture_from_record(record)
    source = getproperty(record, :source)
    return ConformanceFixture(
        id = string(getproperty(record, :id)),
        status = string(getproperty(record, :status)),
        kind = string(getproperty(record, :kind)),
        consumers = _string_vector(getproperty(record, :consumers)),
        source = source === nothing ? nothing : string(source),
        assertions = _string_vector(getproperty(record, :assertions)),
    )
end

function _fixture_from_record(record::AbstractDict)
    source = get(record, "source", get(record, :source, nothing))
    return ConformanceFixture(
        id = string(get(record, "id", get(record, :id, ""))),
        status = string(get(record, "status", get(record, :status, ""))),
        kind = string(get(record, "kind", get(record, :kind, ""))),
        consumers = _string_vector(get(record, "consumers", get(record, :consumers, String[]))),
        source = source === nothing ? nothing : string(source),
        assertions = _string_vector(get(record, "assertions", get(record, :assertions, String[]))),
    )
end

function _fixtures(records)
    return [_fixture_from_record(record) for record in records]
end

"""
    binding_fixture_ids(records; track_id = "08")

Return fixture ids that list the Julia binding track as a consumer.
"""
function binding_fixture_ids(records; track_id = JULIA_BINDING_TRACK_ID)
    return [
        fixture.id for fixture in _fixtures(records) if string(track_id) in fixture.consumers
    ]
end

"""
    ready_fixture_ids(records; track_id = "08")

Return ready fixture ids consumed by the Julia binding track.
"""
function ready_fixture_ids(records; track_id = JULIA_BINDING_TRACK_ID)
    return [
        fixture.id for fixture in _fixtures(records) if
        string(track_id) in fixture.consumers && fixture.status == "ready"
    ]
end

"""
    fixture_status(records, fixture_id)

Return the status for one fixture id, or `nothing` when the id is absent.
"""
function fixture_status(records, fixture_id)
    wanted = string(fixture_id)
    for fixture in _fixtures(records)
        fixture.id == wanted && return fixture.status
    end
    return nothing
end

"""
    conformance_report(records; track_id = "08")

Summarise fixture readiness for the Julia binding without running native FFI or
claiming planned fixtures are implemented.
"""
function conformance_report(records; track_id = JULIA_BINDING_TRACK_ID)
    consumed = [
        fixture for fixture in _fixtures(records) if string(track_id) in fixture.consumers
    ]
    ready = [fixture.id for fixture in consumed if fixture.status == "ready"]
    planned = [fixture.id for fixture in consumed if fixture.status != "ready"]
    return (
        track_id = string(track_id),
        consumed = [fixture.id for fixture in consumed],
        ready = ready,
        planned = planned,
        ready_count = length(ready),
        planned_count = length(planned),
    )
end

function ffi_status()
    return (
        configured = false,
        library = nothing,
        reason = "Native KairoECS FFI library artifact is not configured for the Julia binding yet.",
    )
end

is_ffi_configured() = ffi_status().configured

function self_check()
    return Dict(
        :package => "KairoECS",
        :version => VERSION_STRING,
        :status => "ok",
        :ffi_configured => string(is_ffi_configured()),
    )
end

end # module

module KairoECS

export EventLogRecord,
    EventLogBatch,
    arrow_event_log_schema,
    binding_fixture_ids,
    conformance_report,
    ConformanceFixture,
    from_smoke_bytes,
    fixture_status,
    ffi_status,
    is_ffi_configured,
    ordered_events,
    ready_fixture_ids,
    self_check,
    to_smoke_bytes,
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
    !isempty(strip(event.run_id)) || throw(ArgumentError("run_id must not be empty"))
    !isempty(strip(event.event_id)) || throw(ArgumentError("event_id must not be empty"))
    event.time_scale == "ticks" || throw(ArgumentError("time_scale must be ticks"))
    !isempty(strip(event.event_kind)) || throw(ArgumentError("event_kind must not be empty"))
    event.status in ("dispatched", "cancelled", "skipped", "error") ||
        throw(ArgumentError("unsupported event status: $(event.status)"))
    if event.payload_ref !== nothing && isempty(strip(event.payload_ref))
        throw(ArgumentError("payload_ref must not be empty when present"))
    end
    return event
end

"""
    EventLogBatch(records)

Validated event-log batch for the Track 04 `kairo_ecs.event_log.v1` boundary.
The smoke-byte codec mirrors the Rust/Python lightweight Arrow gate shape while
the native Arrow.jl IPC path is deferred until Julia tooling is available.
"""
struct EventLogBatch
    records::Vector{EventLogRecord}

    function EventLogBatch(records)
        validated = [_validate_event(record) for record in records]
        return new(collect(validated))
    end
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

function Base.:(==)(left::EventLogBatch, right::EventLogBatch)
    return left.records == right.records
end

function _escape_cell(value)
    text = string(value)
    return replace(replace(replace(text, "\\" => "\\\\"), "\t" => "\\t"), "\n" => "\\n")
end

function _unescape_cell(value::AbstractString)
    output = IOBuffer()
    index = firstindex(value)
    while index <= lastindex(value)
        char = value[index]
        if char == '\\' && index < lastindex(value)
            index = nextind(value, index)
            escaped = value[index]
            if escaped == 't'
                print(output, '\t')
            elseif escaped == 'n'
                print(output, '\n')
            elseif escaped == '\\'
                print(output, '\\')
            else
                print(output, '\\')
                print(output, escaped)
            end
        else
            print(output, char)
        end
        index = nextind(value, index)
    end
    return String(take!(output))
end

function _uint128_le_hex(value::UInt128)
    bytes = UInt8[(value >> (8 * offset)) & 0xff for offset in 0:15]
    return bytes2hex(bytes)
end

function _parse_uint128_le_hex(value::AbstractString)
    bytes = hex2bytes(value)
    length(bytes) == 16 || throw(ArgumentError("time_ticks must be 16 little-endian bytes"))
    result = UInt128(0)
    for (offset, byte) in enumerate(bytes)
        result |= UInt128(byte) << (8 * (offset - 1))
    end
    return result
end

function _payload_text(payload::AbstractVector{UInt8})
    return String(copy(payload))
end

function _payload_text(payload::AbstractString)
    return String(payload)
end

"""
    to_smoke_bytes(batch)

Serialize an event-log batch to the repository's dependency-light Arrow smoke
payload. This is a deterministic table-shaped gate, not a replacement for
Arrow.jl IPC once the native Julia package lane is available.
"""
function to_smoke_bytes(batch::EventLogBatch)
    lines = [
        "stream=$(EVENT_LOG_STREAM);schema_version=$(EVENT_LOG_SCHEMA_VERSION)",
        "schema_version\trun_id\tevent_id\tentity_id\ttime_ticks_le_hex\ttime_scale\tpriority\tsequence\tevent_kind\tstatus\tpayload_ref",
    ]
    for record in ordered_events(batch.records)
        push!(
            lines,
            join(
                [
                    string(EVENT_LOG_SCHEMA_VERSION),
                    _escape_cell(record.run_id),
                    _escape_cell(record.event_id),
                    _escape_cell(something(record.entity_id, "")),
                    _uint128_le_hex(record.time_ticks),
                    _escape_cell(record.time_scale),
                    string(record.priority),
                    string(record.sequence),
                    _escape_cell(record.event_kind),
                    _escape_cell(record.status),
                    _escape_cell(something(record.payload_ref, "")),
                ],
                "\t",
            ),
        )
    end
    return Vector{UInt8}(codeunits(join(lines, "\n") * "\n"))
end

"""
    from_smoke_bytes(payload)

Deserialize the dependency-light Arrow smoke payload produced by
`to_smoke_bytes`.
"""
function from_smoke_bytes(payload)
    lines = split(_payload_text(payload), '\n'; keepempty = false)
    expected_header = "stream=$(EVENT_LOG_STREAM);schema_version=$(EVENT_LOG_SCHEMA_VERSION)"
    length(lines) >= 2 && lines[1] == expected_header ||
        throw(ArgumentError("unexpected stream header"))
    expected_fields =
        "schema_version\trun_id\tevent_id\tentity_id\ttime_ticks_le_hex\ttime_scale\tpriority\tsequence\tevent_kind\tstatus\tpayload_ref"
    lines[2] == expected_fields || throw(ArgumentError("unexpected field header"))

    records = EventLogRecord[]
    for line in lines[3:end]
        cells = split(line, '\t'; keepempty = true)
        length(cells) == 11 || throw(ArgumentError("expected 11 cells, got $(length(cells))"))
        parse(UInt16, cells[1]) == EVENT_LOG_SCHEMA_VERSION ||
            throw(ArgumentError("unsupported schema_version: $(cells[1])"))
        entity_id = isempty(cells[4]) ? nothing : _unescape_cell(cells[4])
        payload_ref = isempty(cells[11]) ? nothing : _unescape_cell(cells[11])
        push!(
            records,
            EventLogRecord(
                run_id = _unescape_cell(cells[2]),
                event_id = _unescape_cell(cells[3]),
                entity_id = entity_id,
                time_ticks = _parse_uint128_le_hex(cells[5]),
                time_scale = _unescape_cell(cells[6]),
                priority = parse(Int32, cells[7]),
                sequence = parse(UInt64, cells[8]),
                event_kind = _unescape_cell(cells[9]),
                status = _unescape_cell(cells[10]),
                payload_ref = payload_ref,
            ),
        )
    end
    return EventLogBatch(records)
end

function _string_vector(values)
    return String[string(value) for value in values]
end

function _fixture_from_record(record::ConformanceFixture)
    return record
end

function _record_property(record, name::Symbol, default)
    return hasproperty(record, name) ? getproperty(record, name) : default
end

function _fixture_from_record(record)
    source = _record_property(record, :source, nothing)
    return ConformanceFixture(
        id = string(getproperty(record, :id)),
        status = string(getproperty(record, :status)),
        kind = string(getproperty(record, :kind)),
        consumers = _string_vector(getproperty(record, :consumers)),
        source = source === nothing ? nothing : string(source),
        assertions = _string_vector(_record_property(record, :assertions, String[])),
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

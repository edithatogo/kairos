module KairoECS

export EventLogRecord,
    arrow_event_log_schema,
    ffi_status,
    is_ffi_configured,
    ordered_events,
    self_check,
    version_string

const VERSION_STRING = "0.1.0"
const EVENT_LOG_STREAM = "kairo_ecs.event_log.v1"
const EVENT_LOG_SCHEMA_VERSION = UInt16(1)

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

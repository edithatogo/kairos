use kairo_ecs_arrow::{EventLogBatch, EventLogRecord};
use kairo_ecs_types::{DispatchedEvent, EventId, EventKind, SimTime};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event = DispatchedEvent {
        id: EventId {
            index: 1,
            generation: 0,
        },
        at: SimTime::from_ticks(10),
        priority: 1,
        sequence: 0,
        entity: None,
        kind: EventKind::Custom(3),
    };

    let batch = EventLogBatch::new(vec![EventLogRecord::dispatched("example-run", event)])?;
    let bytes = batch.to_smoke_bytes();
    let decoded = EventLogBatch::from_smoke_bytes(&bytes)?;

    assert_eq!(decoded, batch);
    println!(
        "round-tripped {} event-log record(s) for kairo_ecs.event_log.v1",
        decoded.records().len()
    );

    Ok(())
}

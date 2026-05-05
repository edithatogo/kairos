use kairo_ecs_types::{EntityId, SimTime};
use kairo_ecs_viz::{render_headless, FrameEntity, RenderFrame};

fn main() {
    let frame = RenderFrame::new(SimTime::from_ticks(12))
        .with_entity(FrameEntity::new(entity(1), "triage", 0, 0))
        .with_entity(FrameEntity::new(entity(2), "bed-12", 1250, 500));

    let summary = render_headless(&frame).expect("headless render summary");

    println!(
        "frame={} entities={} events={} bounds={:?}",
        summary.at_ticks, summary.entity_count, summary.event_count, summary.bounds
    );
}

fn entity(index: u64) -> EntityId {
    EntityId {
        index,
        generation: 0,
    }
}

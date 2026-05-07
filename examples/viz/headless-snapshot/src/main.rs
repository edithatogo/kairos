use kairo_ecs_state::World;
use kairo_ecs_types::SimTime;
use kairo_ecs_viz::{
    render_fixture_json, render_headless, render_headless_svg, render_headless_text, RenderFrame,
};

fn main() {
    let mut world = World::new();
    world.spawn();
    world.spawn();

    let frame = RenderFrame::from_world_snapshot(SimTime::from_ticks(12), &world.snapshot());

    let summary = render_headless(&frame).expect("headless render summary");

    println!(
        "frame={} entities={} events={} bounds={:?}",
        summary.at_ticks, summary.entity_count, summary.event_count, summary.bounds
    );
    println!("{}", render_headless_text(&frame).expect("headless render text"));
    println!("{}", render_fixture_json(frame.clone()).expect("visualization fixture"));
    println!("{}", render_headless_svg(&frame).expect("headless svg"));
}

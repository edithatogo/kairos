use kairo_ecs_des::{DESContext, Resource};
use kairo_ecs_types::*;

#[test]
fn factory_bottleneck_smoke() {
    let mut sim = DESContext::new(42);
    sim.add_resource("machine", 1);
    for i in 0..5 {
        sim.schedule_at(SimTime::from_ticks(0), 0, EventKind::Custom(i));
    }
    let outcome = sim.run_for(10);
    assert!(matches!(outcome, StepOutcome::Dispatched(_)));
}

#[test]
fn resource_queue_fifo() {
    let mut r = Resource::new("w", 2);
    let a: Vec<_> = (0..4).map(|i| EntityId::new(i, 0)).collect();
    assert!(r.request(a[0]));
    assert!(r.request(a[1]));
    assert!(!r.request(a[2]));
    assert!(!r.request(a[3]));
    assert_eq!(r.release(), Some(a[2]));
    assert_eq!(r.release(), Some(a[3]));
    assert_eq!(r.release(), None);
    assert_eq!(r.release(), None);
    assert_eq!(r.available_count(), 2);
}

#[test]
fn parallel_resources() {
    let mut sim = DESContext::new(1);
    sim.add_resource("assembly", 3);
    sim.add_resource("testing", 1);
    assert_eq!(sim.resource("assembly").unwrap().available_count(), 3);
}

#[test]
fn resource_not_found() {
    let sim = DESContext::new(0);
    assert!(sim.resource("x").is_none());
}

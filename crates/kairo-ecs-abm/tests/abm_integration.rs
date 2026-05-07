use kairo_ecs_abm::ABMContext;
use kairo_ecs_types::StepOutcome;
use kairo_ecs_types::*;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Pos {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Health {
    value: f64,
}

#[test]
fn agent_with_components() {
    let mut ctx = ABMContext::new(42);
    let a = ctx.spawn_agent();
    ctx.attach(a, Pos { x: 0.0, y: 0.0 });
    ctx.attach(a, Health { value: 100.0 });
    assert_eq!(ctx.get::<Pos>(a), Some(&Pos { x: 0.0, y: 0.0 }));
    assert_eq!(ctx.get::<Health>(a), Some(&Health { value: 100.0 }));
}

#[test]
fn schedule_and_dispatch() {
    let mut ctx = ABMContext::new(7);
    for _ in 0..10 {
        ctx.spawn_agent();
    }
    let agent = ctx.spawn_agent();
    ctx.schedule_behaviour(agent, 1, SimTime::from_ticks(10));
    assert!(matches!(ctx.step(), StepOutcome::Dispatched(_)));
}

#[test]
fn multi_agent_scheduling() {
    let mut ctx = ABMContext::new(1);
    let agents: Vec<_> = (0..10).map(|_| ctx.spawn_agent()).collect();
    for (i, a) in agents.iter().enumerate() {
        ctx.schedule_behaviour(*a, 1, SimTime::from_ticks(i as u128));
    }
    assert_eq!(ctx.run_for(10), 10);
    assert!(matches!(ctx.step(), StepOutcome::Empty));
}

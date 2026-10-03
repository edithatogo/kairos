use kairo_ecs_des::{FlowCallbackSnapshot, FlowCommandSink, FlowError, FlowRuntime, FlowWorldView};
use kairo_ecs_types::{EventKind, SimDuration, SimTime};
fn view(_: &mut (), _: &FlowCallbackSnapshot, _: FlowWorldView<'_>, _: &mut FlowCommandSink) {}
#[test]
fn actors_and_resources_do_not_close_registration_phase() {
    let mut f = FlowRuntime::new();
    assert_eq!(f.ensure_pre_work_registration(), Ok(()));
    f.spawn_actor().unwrap();
    f.create_resource(1).unwrap();
    assert_eq!(f.ensure_pre_work_registration(), Ok(()));
}
#[test]
fn failed_creation_does_not_close_registration_phase() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    assert_eq!(
        f.create_work(a, SimDuration::ZERO, "", ()),
        Err(FlowError::InvalidWork)
    );
    assert_eq!(f.ensure_pre_work_registration(), Ok(()));
}
#[test]
fn successful_task_closes_global_registration_phase() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    f.create_work(a, SimDuration::ZERO, "task", ()).unwrap();
    assert_eq!(
        f.ensure_pre_work_registration(),
        Err(FlowError::InvalidWork)
    );
}
#[test]
fn cleanup_does_not_reopen_registration_phase() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    f.create_work(a, SimDuration::ZERO, "task", ()).unwrap();
    f.despawn_actor_at_with_scheduler_priority(a, SimTime::ZERO, 0)
        .unwrap();
    f.step().unwrap().unwrap();
    assert_eq!(
        f.ensure_pre_work_registration(),
        Err(FlowError::InvalidWork)
    );
}
#[test]
fn existing_domain_registration_semantics_remain_available() {
    let mut f = FlowRuntime::new();
    let a = f.spawn_actor().unwrap();
    f.create_work(a, SimDuration::ZERO, "task", ()).unwrap();
    assert_eq!(
        f.ensure_pre_work_registration(),
        Err(FlowError::InvalidWork)
    );
    f.register_domain_view_hook::<()>("later", EventKind::custom(8010), view)
        .unwrap();
}

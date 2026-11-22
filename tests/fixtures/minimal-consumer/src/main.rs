use kairo_ecs_core::Scheduler;
use kairo_ecs_des::Resource;
use kairo_ecs_state::{ComponentStore, World};
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

#[derive(Debug, PartialEq, Eq)]
enum JobState {
    Waiting,
    InService,
    Completed,
}

fn main() {
    let mut world = World::new();
    let first_job = world.spawn();
    let waiting_job = world.spawn();
    let mut jobs = ComponentStore::<JobState>::new();
    let mut station = Resource::new("service station", 1);

    assert!(station.request(first_job));
    assert!(jobs.insert(first_job, JobState::InService));
    assert!(!station.request(waiting_job));
    assert!(jobs.insert(waiting_job, JobState::Waiting));
    assert_eq!(station.queue_length(), 1);
    assert_eq!(station.available_count(), 0);

    let mut scheduler = Scheduler::new();
    let completion_time = SimTime::from_ticks(5);
    let completion_kind = EventKind::custom(1);
    let completion_id = scheduler.schedule(ScheduleRequest {
        at: completion_time,
        priority: 0,
        kind: completion_kind,
        entity: Some(first_job),
    });

    let StepOutcome::Dispatched(completion) = scheduler.step() else {
        panic!("expected the scheduled service completion");
    };
    assert_eq!(completion.id, completion_id);
    assert_eq!(completion.at, completion_time);
    assert_eq!(completion.kind, completion_kind);
    assert_eq!(completion.entity, Some(first_job));
    let completed_job = completion.entity.expect("completion has a job");
    assert!(world.is_alive(completed_job));
    assert!(jobs.insert(completed_job, JobState::Completed));

    let admitted_job = station.release().expect("the waiting job is admitted");
    assert_eq!(admitted_job, waiting_job);
    assert!(world.is_alive(admitted_job));
    assert!(jobs.insert(admitted_job, JobState::InService));
    assert_eq!(jobs.get(completed_job), Some(&JobState::Completed));
    assert_eq!(jobs.get(admitted_job), Some(&JobState::InService));
    assert_eq!(station.queue_length(), 0);
    assert_eq!(station.available_count(), 0);
    assert_eq!(scheduler.step(), StepOutcome::Empty);

    println!("Dispatched a service completion and admitted the waiting job.");
}

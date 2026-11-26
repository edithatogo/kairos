// SPDX-License-Identifier: Apache-2.0
//! Drive a staged resource allocator through the native debugger.
//! Each debugger event dispatches one live action in the allocator.

use std::num::NonZeroUsize;

use kairo_ecs_core::Scheduler;
use kairo_ecs_debug::ReplaySession;
use kairo_ecs_des::{
    StagedRequestId, StagedRequestState, StagedResourceRuntime, StagedRuntimeError,
};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime};

struct AllocatorModel {
    allocator: StagedResourceRuntime,
    resource: EntityId,
    requests: [EntityId; 3],
    wakeup: Option<EventId>,
}

impl Clone for AllocatorModel {
    fn clone(&self) -> Self {
        // Store entity IDs in the model so restored runtimes can bind local handles.
        Self {
            allocator: StagedResourceRuntime::from_checkpoint_json(&self.checkpoint())
                .expect("restore live allocator checkpoint"),
            resource: self.resource,
            requests: self.requests,
            wakeup: self.wakeup,
        }
    }
}

impl AllocatorModel {
    fn checkpoint(&self) -> Vec<u8> {
        self.allocator
            .checkpoint_json()
            .expect("encode live allocator checkpoint")
    }

    fn request(&self, entity: EntityId) -> StagedRequestId {
        self.allocator.request_by_entity(entity).unwrap()
    }

    fn schedule_next(&mut self, scheduler: &mut Scheduler) {
        if let Some(wakeup) = self.wakeup.take() {
            assert!(scheduler.cancel(wakeup));
        }
        self.wakeup = self.allocator.next_event_time().map(|at| {
            scheduler.schedule(ScheduleRequest {
                at,
                priority: 0,
                entity: None,
                kind: EventKind::custom(0),
            })
        });
    }

    fn dispatch(&mut self, scheduler: &mut Scheduler, event: DispatchedEvent) {
        assert_eq!(self.wakeup.take(), Some(event.id));
        assert_eq!(self.allocator.next_event_time(), Some(event.at));
        assert_eq!(self.allocator.run_for(1), Ok(1));
        self.schedule_next(scheduler);
    }

    fn check_resource(&self, available: u64, active: &[EntityId], queued: &[EntityId]) {
        let resource = self.allocator.resource_by_entity(self.resource).unwrap();
        let state = self.allocator.resource(resource).unwrap();
        assert_eq!(state.capacity, 3);
        assert_eq!(state.available, available);
        assert_eq!(
            state
                .active
                .iter()
                .map(|lease| lease.request_id().entity_id())
                .collect::<Vec<_>>(),
            active,
        );
        assert_eq!(
            state
                .queued
                .iter()
                .map(|request| request.entity_id())
                .collect::<Vec<_>>(),
            queued,
        );
    }
}

fn tick(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

fn scenario() -> ReplaySession<AllocatorModel> {
    let mut allocator = StagedResourceRuntime::new().unwrap();
    let resource = allocator.create_resource(3).unwrap();
    let [a, b, c] = std::array::from_fn(|_| allocator.spawn_actor().unwrap());
    let holder = allocator.submit_quantity(resource, a, tick(0), 2).unwrap();
    assert_eq!(allocator.run_for(1), Ok(1));
    let lease = allocator.request(holder).unwrap().lease.unwrap();
    allocator.release(lease, tick(8)).unwrap();
    let head = allocator.submit_quantity(resource, b, tick(2), 2).unwrap();
    let tail = allocator.submit(resource, c, tick(2)).unwrap();

    let mut model = AllocatorModel {
        allocator,
        resource: resource.entity_id(),
        requests: [holder, head, tail].map(StagedRequestId::entity_id),
        wakeup: None,
    };
    let mut scheduler = Scheduler::new();
    model.schedule_next(&mut scheduler);
    ReplaySession::new(
        scheduler,
        model,
        AllocatorModel::dispatch,
        NonZeroUsize::new(2).unwrap(),
        NonZeroUsize::new(3).unwrap(),
    )
}

fn verify_rewind_and_replay() {
    let mut session = scenario();
    let [holder, head, tail] = session.model().requests;
    session.model().check_resource(1, &[holder], &[]);
    let first = session.step().unwrap().unwrap();
    assert_eq!(first.at, tick(2));
    session.model().check_resource(1, &[holder], &[head]);
    let model = session.model();
    assert_eq!(
        model.allocator.request(model.request(tail)).unwrap().state,
        StagedRequestState::Scheduled,
    );
    let after_first = model.checkpoint();

    let second = session.step().unwrap().unwrap();
    assert_eq!(second.at, tick(2));
    session.model().check_resource(1, &[holder], &[head, tail]);
    let after_second = session.model().checkpoint();

    let release = session.step().unwrap().unwrap();
    assert_eq!(release.at, tick(8));
    session.model().check_resource(0, &[head, tail], &[]);
    let after_release = session.model().checkpoint();
    assert_eq!(session.step().unwrap(), None);

    // Position one lies between saved checkpoints and inside a same-tick pair.
    session.seek(1).unwrap();
    assert_eq!(session.current_event(), Some(first));
    assert_eq!(session.model().checkpoint(), after_first);
    for (event, checkpoint) in [(second, after_second), (release, after_release)] {
        assert_eq!(session.step().unwrap(), Some(event));
        assert_eq!(session.model().checkpoint(), checkpoint);
    }
    assert_eq!(session.step().unwrap(), None);
}

fn verify_fork_isolation() {
    let mut parent = scenario();
    parent.step().unwrap().unwrap();
    parent.step().unwrap().unwrap();
    let model = parent.model();
    let [holder, head, tail] = model.requests;
    let parent_lease = model
        .allocator
        .request(model.request(holder))
        .unwrap()
        .lease
        .unwrap();
    let parent_checkpoint = model.checkpoint();

    let mut branch = parent.fork(|model, scheduler| {
        assert_eq!(
            model.allocator.release(parent_lease, tick(4)),
            Err(StagedRuntimeError::InvalidState),
        );
        assert!(model
            .allocator
            .close_request(model.request(head), tick(4))
            .unwrap());
        model.schedule_next(scheduler);
    });

    assert_eq!(branch.step().unwrap().unwrap().at, tick(4));
    branch.model().check_resource(0, &[holder, tail], &[]);
    let model = branch.model();
    assert_eq!(
        model.allocator.request(model.request(head)).unwrap().state,
        StagedRequestState::Released,
    );
    assert_eq!(branch.step().unwrap().unwrap().at, tick(8));
    branch.model().check_resource(2, &[tail], &[]);
    assert_eq!(branch.step().unwrap(), None);

    assert_eq!(parent.model().checkpoint(), parent_checkpoint);
    parent.model().check_resource(1, &[holder], &[head, tail]);
    assert_eq!(parent.step().unwrap().unwrap().at, tick(8));
    parent.model().check_resource(0, &[head, tail], &[]);
    assert_eq!(parent.step().unwrap(), None);
}

fn main() {
    verify_rewind_and_replay();
    println!("Staged allocator rewind and replay passed.");
    verify_fork_isolation();
    println!("Staged allocator fork isolation passed.");
}

#[test]
fn rewinds_and_replays_the_live_staged_allocator() {
    verify_rewind_and_replay();
}

#[test]
fn forks_the_live_staged_allocator_with_isolated_requests_and_leases() {
    verify_fork_isolation();
}

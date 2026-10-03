//! Shared agent behavior on the authoritative Flow runtime.
use kairo_ecs_des::{
    FlowCallbackSnapshot, FlowCommandSink, FlowError, FlowRuntime, FlowWorldView, WorkId,
};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, EventId, EventKind, SimTime};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowAgentHandle {
    actor: EntityId,
    carrier: WorkId,
    kind: EventKind,
}
pub struct FlowAgentContext<'a, C> {
    pub agent: EntityId,
    pub event: &'a FlowCallbackSnapshot,
    pub state: &'a mut C,
    pub rng: &'a mut DeterministicStream,
    pub view: FlowWorldView<'a>,
    pub commands: &'a mut FlowCommandSink,
}
pub trait FlowAgentBehavior<C> {
    fn update(&mut self, context: FlowAgentContext<'_, C>);
}
struct FlowAgentState<C, B> {
    actor: EntityId,
    state: C,
    behavior: B,
    rng: DeterministicStream,
}
fn dispatch_flow_agent<'a, C, B: FlowAgentBehavior<C>>(
    row: &'a mut FlowAgentState<C, B>,
    event: &'a FlowCallbackSnapshot,
    view: FlowWorldView<'a>,
    commands: &'a mut FlowCommandSink,
) {
    let FlowAgentState {
        actor,
        state,
        behavior,
        rng,
    } = row;
    behavior.update(FlowAgentContext {
        agent: *actor,
        event,
        state,
        rng,
        view,
        commands,
    });
}
pub fn register_flow_agent_behavior<C: 'static, B: FlowAgentBehavior<C> + 'static>(
    flow: &mut FlowRuntime,
    registration: &str,
    kind: EventKind,
) -> Result<(), FlowError> {
    flow.ensure_pre_work_registration()?;
    flow.register_domain_view_hook::<FlowAgentState<C, B>>(
        registration,
        kind,
        dispatch_flow_agent::<C, B>,
    )
}
pub fn create_flow_agent<C: 'static, B: FlowAgentBehavior<C> + 'static>(
    flow: &mut FlowRuntime,
    actor: EntityId,
    registration: &str,
    kind: EventKind,
    run_seed: u64,
    state: C,
    behavior: B,
) -> Result<FlowAgentHandle, FlowError> {
    let row = FlowAgentState {
        actor,
        state,
        behavior,
        rng: DeterministicStream::from_entity(run_seed, actor),
    };
    let carrier = flow.create_actor_domain_context(actor, registration, kind, row)?;
    Ok(FlowAgentHandle {
        actor,
        carrier,
        kind,
    })
}
pub fn schedule_flow_agent_update(
    flow: &mut FlowRuntime,
    agent: FlowAgentHandle,
    at: SimTime,
    scheduler_priority: i32,
) -> Result<EventId, FlowError> {
    if flow.actor_domain_context(agent.actor)? != agent.carrier
        || flow.work(agent.carrier)?.owner != agent.actor
    {
        return Err(FlowError::InvalidWork);
    }
    flow.schedule_domain(agent.carrier, agent.kind, at, scheduler_priority)
}
#[cfg(test)]
mod tests {
    use super::*;
    use kairo_ecs_des::FlowConfig;
    use std::num::NonZeroU64;
    struct Draw {
        calls: u32,
    }
    impl FlowAgentBehavior<Vec<u64>> for Draw {
        fn update(&mut self, c: FlowAgentContext<'_, Vec<u64>>) {
            self.calls += 1;
            c.state.push(c.rng.next_u64());
        }
    }
    #[test]
    fn preconsume_budget_preserves_actual_private_stream_state_and_behavior() {
        let mut f = FlowRuntime::with_config(FlowConfig {
            max_same_tick_flow_transitions: NonZeroU64::new(1).unwrap(),
        });
        let kind = EventKind::custom(9010);
        register_flow_agent_behavior::<Vec<u64>, Draw>(&mut f, "draw", kind).unwrap();
        let actor = f.spawn_actor().unwrap();
        let agent = create_flow_agent(
            &mut f,
            actor,
            "draw",
            kind,
            17,
            Vec::<u64>::new(),
            Draw { calls: 0 },
        )
        .unwrap();
        let initial = f
            .work_context::<FlowAgentState<Vec<u64>, Draw>>(agent.carrier)
            .unwrap();
        assert_eq!(
            initial.rng.clone().into_inner(),
            DeterministicStream::from_entity(17, actor).into_inner()
        );
        assert!(initial.state.is_empty());
        schedule_flow_agent_update(&mut f, agent, SimTime::from_ticks(2), 0).unwrap();
        let pending = schedule_flow_agent_update(&mut f, agent, SimTime::from_ticks(2), 0).unwrap();
        assert!(f.step().unwrap().unwrap().error.is_none());
        let before = f
            .work_context::<FlowAgentState<Vec<u64>, Draw>>(agent.carrier)
            .unwrap();
        let state = before.state.clone();
        let raw = before.rng.clone().into_inner();
        let calls = before.behavior.calls;
        let stats = f.budget_snapshot().scheduler;
        for _ in 0..2 {
            assert_eq!(
                f.step().unwrap_err(),
                FlowError::SameTickBudgetExceeded {
                    at_ticks: 2,
                    limit: 1
                }
            );
            let after = f
                .work_context::<FlowAgentState<Vec<u64>, Draw>>(agent.carrier)
                .unwrap();
            assert_eq!(after.state, state);
            assert_eq!(after.rng.clone().into_inner(), raw);
            assert_eq!(after.behavior.calls, calls);
            assert_eq!(f.budget_snapshot().scheduler, stats);
            assert_eq!(f.budget_snapshot().halted.unwrap().pending.id, pending);
        }
        assert_eq!(calls, 1);
        assert_eq!(state.len(), 1);
    }
}

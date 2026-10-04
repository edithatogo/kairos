//! Small public-API trace emitter for the Q5.1 SimPy 4.1.2 shared cases.
//! Deliberate Kairos tie/preemption/deadline/scheduler-cancel rules remain local
//! oracles; this example covers only ordinary shared resource behavior.
use kairo_ecs_des::{FlowRuntime, LifecycleTransition, RequestId, RequestState};
use kairo_ecs_types::SimTime;
use std::fmt::Write as _;

#[derive(Clone, Copy)]
struct ClaimSpec {
    name: &'static str,
    at: u64,
    duration: u64,
    priority: i32,
    cancel_at: Option<u64>,
}
#[derive(Clone, Copy)]
struct Claim {
    spec: ClaimSpec,
    id: RequestId,
}
#[derive(Clone, Copy)]
struct Event {
    at: u128,
    op: &'static str,
    request: &'static str,
}
fn t(tick: u64) -> SimTime {
    SimTime::from_ticks(u128::from(tick))
}
fn run_case(id: &'static str, capacity: u32, specs: &[ClaimSpec]) -> Vec<Event> {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().expect("actor");
    let resource = flow.create_resource(capacity).expect("resource");
    let mut claims = Vec::with_capacity(specs.len());
    for spec in specs {
        let request = flow
            .acquire(resource)
            .owner(owner)
            .at(t(spec.at))
            .priority(spec.priority)
            .submit()
            .expect("fixed request is valid");
        if let Some(cancel_at) = spec.cancel_at {
            flow.cancel(request, t(cancel_at))
                .expect("fixed queued cancellation is valid");
        }
        claims.push(Claim {
            spec: *spec,
            id: request,
        });
    }

    let mut events = Vec::new();
    let mut drained = false;
    for _ in 0..256 {
        let Some(dispatch) = flow.step().expect("dispatch") else {
            drained = true;
            break;
        };
        assert!(dispatch.error.is_none(), "{}: {:?}", id, dispatch.error);
        for record in dispatch.records {
            let claim = claims
                .iter()
                .find(|claim| claim.id == record.request)
                .expect("all lifecycle records belong to a named fixed request");
            match record.transition {
                LifecycleTransition::Granted => {
                    events.push(Event {
                        at: record.at.ticks(),
                        op: "grant",
                        request: claim.spec.name,
                    });
                    let lease = record.lease.expect("grant includes active lease");
                    let release_at = claim
                        .spec
                        .at
                        .max(record.at.ticks() as u64)
                        .checked_add(claim.spec.duration)
                        .expect("fixed release tick fits");
                    flow.release(lease, SimTime::from_ticks(u128::from(release_at)))
                        .expect("valid active lease release is schedulable");
                }
                LifecycleTransition::Released => {
                    events.push(Event {
                        at: record.at.ticks(),
                        op: "release",
                        request: claim.spec.name,
                    });
                }
                LifecycleTransition::Cancelled => {
                    events.push(Event {
                        at: record.at.ticks(),
                        op: "cancel",
                        request: claim.spec.name,
                    });
                }
                _ => {}
            }
        }
    }
    assert!(drained, "{}: fixed case exceeded 256 dispatches", id);
    for claim in claims {
        let expected = if claim.spec.cancel_at.is_some() {
            RequestState::Cancelled
        } else {
            RequestState::Released
        };
        assert_eq!(
            flow.request(claim.id)
                .expect("request remains inspectable")
                .state,
            expected
        );
    }
    let snapshot = flow
        .resource(resource)
        .expect("resource remains inspectable");
    assert!(snapshot.active.is_empty() && snapshot.queued.is_empty());
    events
}
fn append_case(out: &mut String, case_id: &str, events: &[Event], first: &mut bool) {
    for event in events {
        if !*first {
            out.push(',');
        }
        *first = false;
        write!(
            out,
            "{{\"case\":\"{}\",\"at\":{},\"op\":\"{}\",\"request\":\"{}\"}}",
            case_id, event.at, event.op, event.request
        )
        .expect("write to string");
    }
}
fn main() {
    let capacity_one = run_case(
        "capacity_1",
        1,
        &[
            ClaimSpec {
                name: "a",
                at: 0,
                duration: 4,
                priority: 0,
                cancel_at: None,
            },
            ClaimSpec {
                name: "b",
                at: 1,
                duration: 2,
                priority: 0,
                cancel_at: None,
            },
            ClaimSpec {
                name: "c",
                at: 2,
                duration: 1,
                priority: 0,
                cancel_at: None,
            },
        ],
    );
    let capacity_two = run_case(
        "capacity_2",
        2,
        &[
            ClaimSpec {
                name: "a",
                at: 0,
                duration: 4,
                priority: 0,
                cancel_at: None,
            },
            ClaimSpec {
                name: "b",
                at: 1,
                duration: 4,
                priority: 0,
                cancel_at: None,
            },
            ClaimSpec {
                name: "c",
                at: 2,
                duration: 2,
                priority: 0,
                cancel_at: None,
            },
        ],
    );
    let priority_fifo = run_case(
        "priority_fifo",
        1,
        &[
            ClaimSpec {
                name: "holder",
                at: 0,
                duration: 4,
                priority: 9,
                cancel_at: None,
            },
            ClaimSpec {
                name: "a",
                at: 1,
                duration: 2,
                priority: 5,
                cancel_at: None,
            },
            ClaimSpec {
                name: "b",
                at: 2,
                duration: 1,
                priority: 1,
                cancel_at: None,
            },
            ClaimSpec {
                name: "c",
                at: 3,
                duration: 1,
                priority: 1,
                cancel_at: None,
            },
        ],
    );
    let queued_cancel = run_case(
        "queued_cancel",
        1,
        &[
            ClaimSpec {
                name: "holder",
                at: 0,
                duration: 5,
                priority: 0,
                cancel_at: None,
            },
            ClaimSpec {
                name: "cancelled",
                at: 1,
                duration: 2,
                priority: 0,
                cancel_at: Some(2),
            },
            ClaimSpec {
                name: "survivor",
                at: 3,
                duration: 2,
                priority: 0,
                cancel_at: None,
            },
        ],
    );
    let manual_release = run_case(
        "manual_release",
        1,
        &[
            ClaimSpec {
                name: "holder",
                at: 0,
                duration: 2,
                priority: 0,
                cancel_at: None,
            },
            ClaimSpec {
                name: "waiter",
                at: 1,
                duration: 2,
                priority: 0,
                cancel_at: None,
            },
        ],
    );

    let mut json = String::from("{\"fixture\":\"queue_simpy_shared_v1\",\"version\":1,\"engine\":\"kairos\",\"engine_version\":\"");
    json.push_str(env!("CARGO_PKG_VERSION"));
    json.push_str("\",\"events\":[");
    let mut first = true;
    append_case(&mut json, "capacity_1", &capacity_one, &mut first);
    append_case(&mut json, "capacity_2", &capacity_two, &mut first);
    append_case(&mut json, "priority_fifo", &priority_fifo, &mut first);
    append_case(&mut json, "queued_cancel", &queued_cancel, &mut first);
    append_case(&mut json, "manual_release", &manual_release, &mut first);
    json.push_str("]}");
    println!("{json}");
}

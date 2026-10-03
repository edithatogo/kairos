#![cfg(feature = "pdes")]

use std::collections::{BTreeMap, VecDeque};

use kairo_ecs_core::Scheduler;
use kairo_ecs_pdes::{ConservativeProcess, ConservativeRuntime, LpId, PartitionPlan, RemoteEvent};
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome};

// Versioned deterministic DES/ABM fixture. Both execution lanes consume the
// same model transition; only the event scheduler differs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Model {
    id: u32,
    count: u32,
    queue: VecDeque<(u64, u64)>,
    completed: Vec<u64>,
    energy: u64,
    credits: u64,
    actions: u64,
}

fn payload(kind: u8, value: u64, remaining: u64) -> Vec<u8> {
    let mut bytes = vec![kind];
    bytes.extend(value.to_le_bytes());
    bytes.extend(remaining.to_le_bytes());
    bytes
}

fn event(
    source: u32,
    destination: u32,
    tick: u128,
    kind: u8,
    value: u64,
    remaining: u64,
) -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(source),
        dest_lp: LpId(destination),
        tick: SimTime::from_ticks(tick),
        event_payload: payload(kind, value, remaining),
    }
}

impl ConservativeProcess for Model {
    fn on_event(&mut self, input: &RemoteEvent) -> Vec<RemoteEvent> {
        let kind = input.event_payload[0];
        let value = u64::from_le_bytes(input.event_payload[1..9].try_into().unwrap());
        let remaining = u64::from_le_bytes(input.event_payload[9..17].try_into().unwrap());
        let now = input.tick.ticks();
        match kind {
            0 => {
                // DES arrival: one server per LP, FIFO waiting queue.
                self.queue.push_back((value, remaining));
                if self.queue.len() == 1 {
                    vec![event(self.id, self.id, now + 3, 1, value, remaining)]
                } else {
                    Vec::new()
                }
            }
            1 => {
                // Service completion, then a delayed arrival at next LP.
                assert_eq!(self.queue.pop_front(), Some((value, remaining)));
                self.completed.push(value);
                let mut output = Vec::new();
                if let Some(&(next, hops)) = self.queue.front() {
                    output.push(event(self.id, self.id, now + 3, 1, next, hops));
                }
                if remaining > 0 {
                    output.push(event(
                        self.id,
                        (self.id + 1) % self.count,
                        now + 2,
                        0,
                        value,
                        remaining - 1,
                    ));
                }
                output
            }
            2 => {
                // ABM action: owned energy evolves; agent sends a credit.
                self.energy = self
                    .energy
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(value);
                self.actions += 1;
                let mut output = vec![event(
                    self.id,
                    (self.id + 1) % self.count,
                    now + 2,
                    3,
                    self.energy % 997,
                    0,
                )];
                if remaining > 0 {
                    output.push(event(self.id, self.id, now + 4, 2, value, remaining - 1));
                }
                output
            }
            3 => {
                self.credits += value;
                Vec::new()
            }
            _ => unreachable!(),
        }
    }
}

fn initial_events(lp_count: u32, mode: u8, seed: u64) -> Vec<RemoteEvent> {
    let mut events = Vec::new();
    for id in 0..lp_count {
        if mode != 1 {
            for arrival in 0..4 {
                events.push(event(
                    id,
                    id,
                    arrival * 7,
                    0,
                    seed + u64::from(id) * 4 + arrival as u64,
                    3,
                ));
            }
        }
        if mode != 0 {
            events.push(event(id, id, 1, 2, seed + u64::from(id), 40));
        }
    }
    events
}

fn models(count: u32) -> BTreeMap<LpId, Model> {
    (0..count)
        .map(|id| {
            (
                LpId(id),
                Model {
                    id,
                    count,
                    ..Model::default()
                },
            )
        })
        .collect()
}

fn sequential(count: u32, inputs: &[RemoteEvent], horizon: u128) -> BTreeMap<LpId, Model> {
    let mut states = models(count);
    let mut scheduler = Scheduler::new();
    let mut events = BTreeMap::new();
    let mut next = 0u32;
    let schedule = |input: RemoteEvent,
                    scheduler: &mut Scheduler,
                    events: &mut BTreeMap<u32, RemoteEvent>,
                    next: &mut u32| {
        let key = *next;
        *next += 1;
        scheduler.schedule(ScheduleRequest {
            at: input.tick,
            priority: input.source_lp.0 as i32,
            entity: None,
            kind: EventKind::Custom(key),
        });
        events.insert(key, input);
    };
    for input in inputs {
        schedule(input.clone(), &mut scheduler, &mut events, &mut next);
    }
    while let StepOutcome::Dispatched(dispatched) = scheduler.step() {
        if dispatched.at > SimTime::from_ticks(horizon) {
            break;
        }
        let EventKind::Custom(key) = dispatched.kind;
        let input = events.remove(&key).unwrap();
        for output in states.get_mut(&input.dest_lp).unwrap().on_event(&input) {
            schedule(output, &mut scheduler, &mut events, &mut next);
        }
    }
    states
}

#[test]
fn production_runtime_matches_core_scheduler_for_des_abm_and_mixed_models() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/pdes_conservative_parity_v1.json"
    ))
    .unwrap();
    assert_eq!(fixture["fixture"], "pdes_conservative_parity_v1");
    assert_eq!(fixture["version"], 1);
    let horizon = u128::from(fixture["horizon_ticks"].as_u64().unwrap());
    let lookahead = u128::from(fixture["lookahead_ticks"].as_u64().unwrap());
    for count in fixture["lp_counts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
    {
        for name in fixture["workloads"].as_array().unwrap() {
            let mode = match name.as_str().unwrap() {
                "des" => 0,
                "abm" => 1,
                "mixed" => 2,
                _ => panic!("invalid fixture workload"),
            };
            for seed in fixture["seeds"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap())
            {
                let initial = initial_events(count, mode, seed);
                let plan = PartitionPlan::from_entities(
                    count,
                    SimDuration::from_ticks(lookahead),
                    (0..count)
                        .map(|id| EntityId::new(u64::from(id), 0))
                        .collect(),
                )
                .unwrap();
                let topology = (0..count)
                    .map(|id| (LpId(id), vec![LpId((id + 1) % count)]))
                    .collect();
                let mut runtime = ConservativeRuntime::new(plan, topology, models(count)).unwrap();
                for input in &initial {
                    runtime.schedule_initial(input.clone()).unwrap();
                }
                let report = runtime.run_until(SimTime::from_ticks(horizon)).unwrap();
                assert_eq!(
                    runtime.processes(),
                    &sequential(count, &initial, horizon),
                    "count={count}, mode={mode}, seed={seed}"
                );
                assert!(report.processed_events > 0);
                assert_eq!(runtime.gvt(), SimTime::from_ticks(horizon));
                assert!(report.gvt_history.windows(2).all(|pair| pair[0] <= pair[1]));
            }
        }
    }
}

//! Seeded timed-cycle model v1. SplitMix64 is test-local, never engine RNG.
use kairo_ecs_des::{
    FlowRuntime, LifecycleTransition as L, PreemptionStrategy as S, RequestState, WorkHandlers,
    WorkProgress, WorkState,
};
use kairo_ecs_types::{SimDuration, SimTime};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

const SEEDS: [u64; 6] = [0, 1, 7, 42, 0xdeadbeef, u64::MAX];
struct Generator {
    state: u64,
    draws: usize,
}
impl Generator {
    fn draw(&mut self) -> u128 {
        self.draws += 1;
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        u128::from(z ^ (z >> 31))
    }
}
#[derive(Clone, Debug)]
struct Case {
    gaps: Vec<u128>,
    urgent: Vec<u128>,
    starts: Vec<u128>,
    ends: Vec<u128>,
    duration: u128,
    draws: usize,
}
impl Case {
    fn generate(g: &mut Generator) -> Self {
        let n = 2 + (g.draw() % 7) as usize;
        let mut gaps = Vec::new();
        let mut urgent = Vec::new();
        for _ in 0..n {
            gaps.push(1 + g.draw() % 5);
            urgent.push(1 + g.draw() % 5);
        }
        let duration = gaps.iter().sum::<u128>() + 1 + g.draw() % 11;
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        let mut previous = 0;
        for i in 0..n {
            starts.push(previous + gaps[i]);
            previous = starts[i] + urgent[i];
            ends.push(previous);
        }
        Self {
            gaps,
            urgent,
            starts,
            ends,
            duration,
            draws: g.draws,
        }
    }
    // Independent interval intersection: active segments are [previous urgent end, next interruption).
    fn accounting(&self, at: u128, strategy: S) -> (u128, u128, u128) {
        let end = match strategy {
            S::Suspend => self.duration + self.urgent.iter().sum::<u128>(),
            S::Restart => self.ends.last().unwrap() + self.duration,
            S::Abort => self.starts[0],
        };
        let at = at.min(end);
        let mut busy = 0;
        let mut useful = 0;
        let mut start = 0;
        for i in 0..self.gaps.len() {
            let elapsed = at.min(self.starts[i]).saturating_sub(start);
            busy += elapsed;
            useful += elapsed;
            if at < self.starts[i] {
                return (busy, useful, self.duration - useful);
            }
            match strategy {
                S::Abort => return (busy, useful, self.duration - useful),
                S::Restart => useful = 0,
                S::Suspend => {}
            }
            if at < self.ends[i] {
                return (busy, useful, self.duration - useful);
            }
            start = self.ends[i];
        }
        let elapsed = at.saturating_sub(start);
        busy += elapsed;
        useful += elapsed;
        (busy, useful, self.duration - useful)
    }
}
#[derive(Clone)]
struct Context {
    generation: u32,
    factories: Rc<RefCell<usize>>,
    callbacks: Rc<RefCell<Vec<WorkProgress>>>,
}
fn factory(initial: &Context) -> Context {
    *initial.factories.borrow_mut() += 1;
    Context {
        generation: initial.generation + 1,
        factories: initial.factories.clone(),
        callbacks: initial.callbacks.clone(),
    }
}
fn observe(context: &mut Context, progress: &WorkProgress) {
    context.callbacks.borrow_mut().push(progress.clone());
}
fn t(n: u128) -> SimTime {
    SimTime::from_ticks(n)
}
fn d(n: u128) -> SimDuration {
    SimDuration::from_ticks(n)
}

#[test]
fn seeded_timed_cycles_match_independent_interval_model_v1() {
    for seed in SEEDS {
        let mut generator = Generator {
            state: seed,
            draws: 0,
        };
        for case_index in 0..32 {
            let case = Case::generate(&mut generator);
            for strategy in [S::Suspend, S::Restart, S::Abort] {
                let replay = format!("generator=v1 seed={seed} case={case_index} draw={} strategy={strategy:?} inputs={case:?}",case.draws);
                let mut f = FlowRuntime::new();
                let owner = f.spawn_actor().unwrap();
                let resource = f.create_resource(1).unwrap();
                f.register_work_handlers::<Context>(
                    "property.low",
                    WorkHandlers {
                        on_resume: Some(observe),
                        on_restart: Some(observe),
                        on_abort: Some(observe),
                        on_cancel: None,
                    },
                )
                .unwrap();
                let factories = Rc::new(RefCell::new(0));
                let callbacks = Rc::new(RefCell::new(Vec::new()));
                let context = Context {
                    generation: 0,
                    factories: factories.clone(),
                    callbacks: callbacks.clone(),
                };
                let low = if strategy == S::Restart {
                    f.create_restartable_work(
                        owner,
                        d(case.duration),
                        "property.low",
                        context,
                        factory,
                    )
                    .unwrap()
                } else {
                    f.create_work(owner, d(case.duration), "property.low", context)
                        .unwrap()
                };
                let request = f
                    .acquire(resource)
                    .owner(owner)
                    .timed_work(low)
                    .priority(9)
                    .preemptible(strategy)
                    .submit()
                    .unwrap();
                let mut urgent_requests = Vec::new();
                for i in 0..case.gaps.len() {
                    let work = f
                        .create_work(owner, d(case.urgent[i]), &format!("urgent.{i}"), ())
                        .unwrap();
                    urgent_requests.push(
                        f.acquire(resource)
                            .owner(owner)
                            .timed_work(work)
                            .priority(1)
                            .can_preempt(true)
                            .at(t(case.starts[i]))
                            .submit()
                            .unwrap(),
                    );
                }
                let mut records = Vec::new();
                let mut causal = BTreeSet::new();
                let mut drained = false;
                let mut previous_progress = None;
                for dispatch_index in 0..512 {
                    let Some(dispatch) = f.step().unwrap() else {
                        drained = true;
                        break;
                    };
                    assert!(
                        dispatch.error.is_none(),
                        "{replay} dispatch={dispatch_index} {dispatch:?}"
                    );
                    for (ordinal, row) in dispatch.records.iter().enumerate() {
                        assert_eq!(row.causal_event_id, dispatch.event, "{replay}");
                        assert_eq!(row.transition_ordinal as usize, ordinal, "{replay}");
                        assert!(
                            causal.insert((row.causal_event_id, row.transition_ordinal)),
                            "{replay}"
                        );
                        assert_eq!(row.at, dispatch.at, "{replay}");
                    }
                    let snapshot = f.resource(resource).unwrap();
                    assert_eq!(
                        snapshot.available as usize + snapshot.active.len(),
                        1,
                        "{replay}"
                    );
                    assert_eq!(
                        snapshot.active.len(),
                        snapshot.allocations.len(),
                        "{replay}"
                    );
                    let queued: BTreeSet<_> = snapshot.queued.iter().copied().collect();
                    assert_eq!(queued.len(), snapshot.queued.len(), "{replay}");
                    for allocation in &snapshot.allocations {
                        assert!(snapshot.active.contains(&allocation.lease), "{replay}");
                        assert!(!queued.contains(&allocation.request), "{replay}");
                        let q = f.request(allocation.request).unwrap();
                        assert_eq!(q.state, RequestState::Active, "{replay}");
                        assert_eq!(q.lease, Some(allocation.lease), "{replay}");
                    }
                    for q in &queued {
                        assert!(
                            matches!(
                                f.request(*q).unwrap().state,
                                RequestState::Queued | RequestState::Suspended
                            ),
                            "{replay}"
                        );
                    }
                    let progress = f.work_progress(low).unwrap();
                    assert_eq!(
                        progress.useful_elapsed.ticks() + progress.remaining.ticks(),
                        case.duration,
                        "{replay}"
                    );
                    assert!(
                        progress.cumulative_busy >= progress.useful_elapsed,
                        "{replay}"
                    );
                    // Empty dispatches also advance time: compare capped independent accounting.
                    let expected = case.accounting(dispatch.at.ticks(), strategy);
                    assert_eq!(
                        (
                            progress.cumulative_busy.ticks(),
                            progress.useful_elapsed.ticks(),
                            progress.remaining.ticks()
                        ),
                        expected,
                        "{replay} at={:?}",
                        dispatch.at
                    );
                    if dispatch.records.is_empty() {
                        if let Some((last_at, last)) = &previous_progress {
                            if *last_at == dispatch.at {
                                assert_eq!(
                                    &progress, last,
                                    "{replay} empty same-tick dispatch mutated accounting"
                                );
                            }
                        }
                    }
                    previous_progress = Some((dispatch.at, progress));
                    records.extend(dispatch.records);
                }
                assert!(drained, "{replay} event budget exhausted");
                let rows: Vec<_> = records
                    .iter()
                    .filter(|row| row.request == request)
                    .collect();
                let terminal: Vec<_> = rows
                    .iter()
                    .filter(|row| {
                        matches!(
                            row.transition,
                            L::Completed | L::Aborted | L::Released | L::Cancelled | L::TimedOut
                        )
                    })
                    .collect();
                assert_eq!(terminal.len(), 1, "{replay}");
                let n = case.gaps.len();
                let sum_gaps = case.gaps.iter().sum::<u128>();
                let (end, busy, attempt, execution) = match strategy {
                    S::Suspend => (
                        case.duration + case.urgent.iter().sum::<u128>(),
                        case.duration,
                        0,
                        n + 1,
                    ),
                    S::Restart => (
                        case.ends[n - 1] + case.duration,
                        case.duration + sum_gaps,
                        n,
                        n + 1,
                    ),
                    S::Abort => (case.starts[0], case.gaps[0], 0, 1),
                };
                assert_eq!(terminal[0].at, t(end), "{replay}");
                assert_eq!(
                    terminal[0].transition,
                    if strategy == S::Abort {
                        L::Aborted
                    } else {
                        L::Completed
                    },
                    "{replay}"
                );
                let p = f.work_progress(low).unwrap();
                assert_eq!(p.cumulative_busy, d(busy), "{replay}");
                assert_eq!(
                    (p.attempt_revision, p.execution_revision),
                    (attempt as u64, execution as u64),
                    "{replay}"
                );
                assert_eq!(
                    p.state,
                    if strategy == S::Abort {
                        WorkState::Aborted
                    } else {
                        WorkState::Completed
                    },
                    "{replay}"
                );
                assert_eq!(
                    rows.iter().filter(|r| r.transition == L::Granted).count(),
                    1,
                    "{replay}"
                );
                let transitions = match strategy {
                    S::Suspend => L::Resumed,
                    S::Restart => L::Restarted,
                    S::Abort => L::Aborted,
                };
                let interruptions = if strategy == S::Abort { 1 } else { n };
                assert_eq!(
                    rows.iter().filter(|r| r.transition == L::Preempted).count(),
                    interruptions,
                    "{replay}"
                );
                assert_eq!(
                    rows.iter().filter(|r| r.transition == transitions).count(),
                    interruptions,
                    "{replay}"
                );
                assert_eq!(callbacks.borrow().len(), interruptions, "{replay}");
                for (i, callback) in callbacks.borrow().iter().enumerate() {
                    let expected = case.accounting(case.starts[i], strategy);
                    assert_eq!(
                        (
                            callback.cumulative_busy.ticks(),
                            callback.useful_elapsed.ticks(),
                            callback.remaining.ticks()
                        ),
                        expected,
                        "{replay} callback={i}"
                    );
                }
                assert_eq!(
                    *factories.borrow(),
                    if strategy == S::Restart { n + 1 } else { 0 },
                    "{replay}"
                );
                assert_eq!(
                    f.work_context::<Context>(low).unwrap().generation,
                    u32::from(strategy == S::Restart),
                    "{replay}"
                );
                for (i, q) in urgent_requests.iter().enumerate() {
                    let terminals: Vec<_> = records
                        .iter()
                        .filter(|r| {
                            r.request == *q
                                && matches!(
                                    r.transition,
                                    L::Completed
                                        | L::Aborted
                                        | L::Released
                                        | L::Cancelled
                                        | L::TimedOut
                                )
                        })
                        .collect();
                    assert_eq!(terminals.len(), 1, "{replay} urgent={i}");
                    assert_eq!(terminals[0].transition, L::Completed, "{replay} urgent={i}");
                    assert_eq!(terminals[0].at, t(case.ends[i]), "{replay}");
                }
                if strategy == S::Abort {
                    assert!(
                        !rows.iter().any(|r| r.at > t(end)
                            && matches!(r.transition, L::Granted | L::Resumed | L::Restarted)),
                        "{replay}"
                    );
                }
                let snapshot = f.resource(resource).unwrap();
                assert_eq!(
                    (
                        snapshot.available,
                        snapshot.active.len(),
                        snapshot.queued.len()
                    ),
                    (1, 0, 0),
                    "{replay}"
                );
            }
        }
    }
}

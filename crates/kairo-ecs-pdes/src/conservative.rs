//! Single-host conservative PDES runtime with explicit positive lookahead.
//!
//! Each edge carries an exclusive lower bound: an event is safe to process only
//! when its tick is strictly less than every incoming bound. Runtime messages
//! are delivered atomically after all handlers in a parallel round finish.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::thread;

use kairo_ecs_types::{SimDuration, SimTime};

use super::{LpId, PartitionPlan, RemoteEvent, Tick};

/// Event-driven process state owned by one logical process.
pub trait ConservativeProcess: Send {
    /// Apply one input event and return the zero or more events it emits.
    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent>;
}

/// Failure raised while constructing or running a conservative runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    InvalidLookahead,
    ProcessSetMismatch {
        missing: Vec<LpId>,
        unexpected: Vec<LpId>,
    },
    MissingTopologyEntry(LpId),
    UnknownTopologySource(LpId),
    UnknownTopologyDestination {
        source: LpId,
        destination: LpId,
    },
    SelfLoop(LpId),
    DuplicateNeighbor {
        source: LpId,
        destination: LpId,
    },
    UnknownInitialSource(LpId),
    UnknownInitialDestination(LpId),
    InitialRouteMissing {
        source: LpId,
        destination: LpId,
    },
    InitialEventsAlreadyClosed,
    Straggler {
        lp_id: LpId,
        event_tick: Tick,
        local_time: Tick,
    },
    OutboundSourceMismatch {
        emitting_lp: LpId,
        declared_source: LpId,
    },
    UnknownOutboundDestination(LpId),
    OutboundRouteMissing {
        source: LpId,
        destination: LpId,
    },
    LookaheadOverflow {
        lp_id: LpId,
        event_tick: Tick,
        lookahead: SimDuration,
    },
    OutboundLookaheadViolation {
        lp_id: LpId,
        event_tick: Tick,
        outbound_tick: Tick,
        minimum_tick: Tick,
    },
    HorizonOverflow(Tick),
    EventSequenceOverflow,
    EventBudgetExceeded {
        budget: usize,
    },
    SpawnFailed(LpId),
    HorizonRegression {
        previous: Tick,
        requested: Tick,
    },
    HandlerPanicked(LpId),
    Poisoned,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "conservative PDES runtime error: {self:?}")
    }
}

impl std::error::Error for RuntimeError {}

/// Counters and frontier history from one or more `run_until` calls.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeReport {
    /// Number of input events handled by the runtime.
    pub processed_events: u64,
    /// Number of emitted events crossing LP boundaries.
    pub remote_events: u64,
    /// Number of events emitted by handlers, including local follow-ups.
    pub emitted_events: u64,
    /// Number of edge lower-bound advertisements advanced.
    pub null_messages: u64,
    /// Number of conservative scheduling rounds.
    pub rounds: u64,
    /// Monotonic global virtual time snapshots, beginning at zero.
    pub gvt_history: Vec<Tick>,
    /// Maximum simultaneous scoped LP workers in any execution round.
    pub worker_count: usize,
}

type QueueKey = (Tick, LpId, u64);

/// A production in-process conservative runtime. Each logical process has one
/// owned state value and one timestamp-ordered input queue.
pub struct ConservativeRuntime<P: ConservativeProcess> {
    partition: PartitionPlan,
    topology: BTreeMap<LpId, Vec<LpId>>,
    processes: BTreeMap<LpId, P>,
    queues: BTreeMap<LpId, BTreeMap<QueueKey, RemoteEvent>>,
    /// Per-edge exclusive lower bounds advertised by source to destination.
    outbound_bounds: BTreeMap<(LpId, LpId), Tick>,
    /// Greatest exclusive inbound frontier reached by each LP.
    local_horizons: BTreeMap<LpId, Tick>,
    next_sequence: u64,
    initial_events_open: bool,
    poisoned: bool,
    gvt: Tick,
    report: RuntimeReport,
    last_requested_horizon: Option<Tick>,
}

impl<P: ConservativeProcess> ConservativeRuntime<P> {
    /// Construct a runtime after validating partition ownership, processes, and
    /// every directed communication edge.
    pub fn new(
        partition: PartitionPlan,
        topology: BTreeMap<LpId, Vec<LpId>>,
        processes: BTreeMap<LpId, P>,
    ) -> Result<Self, RuntimeError> {
        if partition.lookahead() == SimDuration::ZERO {
            return Err(RuntimeError::InvalidLookahead);
        }
        let lp_ids = partition
            .segments()
            .iter()
            .map(|segment| segment.id)
            .collect::<BTreeSet<_>>();
        let process_ids = processes.keys().copied().collect::<BTreeSet<_>>();
        if lp_ids != process_ids {
            return Err(RuntimeError::ProcessSetMismatch {
                missing: lp_ids.difference(&process_ids).copied().collect(),
                unexpected: process_ids.difference(&lp_ids).copied().collect(),
            });
        }

        for lp_id in &lp_ids {
            if !topology.contains_key(lp_id) {
                return Err(RuntimeError::MissingTopologyEntry(*lp_id));
            }
        }
        for (&source, destinations) in &topology {
            if !lp_ids.contains(&source) {
                return Err(RuntimeError::UnknownTopologySource(source));
            }
            let mut unique = BTreeSet::new();
            for &destination in destinations {
                if !lp_ids.contains(&destination) {
                    return Err(RuntimeError::UnknownTopologyDestination {
                        source,
                        destination,
                    });
                }
                if source == destination {
                    return Err(RuntimeError::SelfLoop(source));
                }
                if !unique.insert(destination) {
                    return Err(RuntimeError::DuplicateNeighbor {
                        source,
                        destination,
                    });
                }
            }
        }

        let initial_bound = SimTime::from_ticks(partition.lookahead().ticks());
        let mut outbound_bounds = BTreeMap::new();
        for (&source, destinations) in &topology {
            for &destination in destinations {
                outbound_bounds.insert((source, destination), initial_bound);
            }
        }
        let queues = lp_ids
            .iter()
            .copied()
            .map(|lp_id| (lp_id, BTreeMap::new()))
            .collect();
        let local_horizons = lp_ids
            .iter()
            .copied()
            .map(|lp_id| (lp_id, SimTime::ZERO))
            .collect();

        Ok(Self {
            partition,
            topology,
            processes,
            queues,
            outbound_bounds,
            local_horizons,
            next_sequence: 0,
            initial_events_open: true,
            poisoned: false,
            gvt: SimTime::ZERO,
            report: RuntimeReport {
                processed_events: 0,
                remote_events: 0,
                emitted_events: 0,
                null_messages: 0,
                rounds: 0,
                gvt_history: vec![SimTime::ZERO],
                worker_count: 0,
            },
            last_requested_horizon: None,
        })
    }

    /// Add an externally supplied event before the first run. Its route and
    /// partition endpoints are validated before it enters the destination queue.
    pub fn schedule_initial(&mut self, event: RemoteEvent) -> Result<(), RuntimeError> {
        self.ensure_healthy()?;
        if !self.initial_events_open {
            return Err(RuntimeError::InitialEventsAlreadyClosed);
        }
        if !self.processes.contains_key(&event.source_lp) {
            return Err(RuntimeError::UnknownInitialSource(event.source_lp));
        }
        if !self.processes.contains_key(&event.dest_lp) {
            return Err(RuntimeError::UnknownInitialDestination(event.dest_lp));
        }
        if !self.has_route(event.source_lp, event.dest_lp) {
            return Err(RuntimeError::InitialRouteMissing {
                source: event.source_lp,
                destination: event.dest_lp,
            });
        }
        self.enqueue(event)?;
        Ok(())
    }

    /// Run until every event at or before `inclusive_horizon` is processed.
    /// An event at a null-message bound remains pending until that bound grows.
    pub fn run_until(&mut self, inclusive_horizon: Tick) -> Result<RuntimeReport, RuntimeError> {
        self.run_until_with_budget(inclusive_horizon, 1_000_000)
    }

    /// Run with a maximum number of input events handled during this call.
    /// If the budget is reached, already completed events remain committed and
    /// the runtime can resume with a later call.
    pub fn run_until_with_budget(
        &mut self,
        inclusive_horizon: Tick,
        event_budget: usize,
    ) -> Result<RuntimeReport, RuntimeError> {
        self.ensure_healthy()?;
        if let Some(previous) = self.last_requested_horizon {
            if inclusive_horizon < previous {
                return Err(RuntimeError::HorizonRegression {
                    previous,
                    requested: inclusive_horizon,
                });
            }
        }
        let exclusive_horizon = inclusive_horizon
            .checked_add(SimDuration::from_ticks(1))
            .ok_or(RuntimeError::HorizonOverflow(inclusive_horizon))?;
        self.initial_events_open = false;
        self.last_requested_horizon = Some(inclusive_horizon);
        let mut events_this_call = 0usize;

        loop {
            let safe = self.compute_safe_frontiers(exclusive_horizon);
            let remaining = event_budget.saturating_sub(events_this_call);
            let batches = self.take_eligible_batches(&safe, remaining);
            let has_work = batches.values().any(|batch| !batch.is_empty());
            let mut advanced = false;
            if has_work {
                let count = batches.values().map(Vec::len).sum::<usize>();
                self.execute_round(batches)?;
                events_this_call += count;
                self.report.rounds = self.report.rounds.saturating_add(1);
            }

            if events_this_call >= event_budget && self.has_eligible_event(&safe) {
                return Err(RuntimeError::EventBudgetExceeded {
                    budget: event_budget,
                });
            }
            if !has_work {
                advanced = self.advance_bounds(&safe, exclusive_horizon);
                if advanced {
                    self.report.rounds = self.report.rounds.saturating_add(1);
                }
            }
            self.refresh_gvt(&safe, inclusive_horizon);
            if !has_work && !advanced && self.is_complete(exclusive_horizon) {
                break;
            }
            if !has_work && !advanced {
                self.poisoned = true;
                return Err(RuntimeError::Poisoned);
            }
        }
        Ok(self.report.clone())
    }

    /// Current global virtual time, bounded by all local horizons and queued events.
    pub fn gvt(&self) -> Tick {
        self.gvt
    }

    /// Read-only access to the process states keyed by logical process id.
    pub fn processes(&self) -> &BTreeMap<LpId, P> {
        &self.processes
    }

    /// Cumulative metrics across all runtime calls.
    pub fn report(&self) -> &RuntimeReport {
        &self.report
    }

    /// Number of events currently waiting in logical-process queues.
    pub fn pending_events(&self) -> usize {
        self.queues.values().map(BTreeMap::len).sum()
    }

    fn ensure_healthy(&self) -> Result<(), RuntimeError> {
        if self.poisoned {
            Err(RuntimeError::Poisoned)
        } else {
            Ok(())
        }
    }

    fn has_route(&self, source: LpId, destination: LpId) -> bool {
        if source == destination {
            return true;
        }
        self.topology
            .get(&source)
            .is_some_and(|destinations| destinations.contains(&destination))
    }

    fn enqueue(&mut self, event: RemoteEvent) -> Result<(), RuntimeError> {
        let sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(RuntimeError::EventSequenceOverflow)?;
        self.queues
            .get_mut(&event.dest_lp)
            .expect("validated destination has a queue")
            .insert((event.tick, event.source_lp, sequence), event);
        Ok(())
    }

    fn compute_safe_frontiers(&self, horizon: Tick) -> BTreeMap<LpId, Tick> {
        self.processes
            .keys()
            .copied()
            .map(|destination| {
                let inbound = self
                    .outbound_bounds
                    .iter()
                    .filter(|((_, target), _)| *target == destination)
                    .map(|(_, bound)| *bound)
                    .min()
                    .unwrap_or(horizon);
                (destination, inbound.min(horizon))
            })
            .collect()
    }

    fn take_eligible_batches(
        &mut self,
        safe: &BTreeMap<LpId, Tick>,
        limit: usize,
    ) -> BTreeMap<LpId, Vec<RemoteEvent>> {
        let mut batches = BTreeMap::new();
        let mut remaining = limit;
        for (&lp_id, queue) in &mut self.queues {
            let bound = safe[&lp_id];
            let keys = queue
                .keys()
                .next()
                .copied()
                .filter(|(tick, _, _)| *tick < bound);
            let events = keys
                .filter(|_| remaining > 0)
                .and_then(|key| queue.remove(&key))
                .into_iter()
                .collect::<Vec<_>>();
            remaining = remaining.saturating_sub(events.len());
            batches.insert(lp_id, events);
        }
        batches
    }

    fn has_eligible_event(&self, safe: &BTreeMap<LpId, Tick>) -> bool {
        self.queues.iter().any(|(lp_id, queue)| {
            queue
                .keys()
                .next()
                .is_some_and(|(tick, _, _)| *tick < safe[lp_id])
        })
    }

    fn execute_round(
        &mut self,
        batches: BTreeMap<LpId, Vec<RemoteEvent>>,
    ) -> Result<(), RuntimeError> {
        let lookahead = self.partition.lookahead();
        let (outcomes, spawn_error) = thread::scope(|scope| {
            let mut handles = Vec::new();
            let mut spawn_error = None;
            for (&lp_id, process) in &mut self.processes {
                let batch = batches.get(&lp_id).cloned().unwrap_or_default();
                if batch.is_empty() {
                    continue;
                }
                let spawn = thread::Builder::new()
                    .name(format!("pdes-lp-{}", lp_id.0))
                    .spawn_scoped(scope, move || {
                        let mut outbound = Vec::new();
                        for event in &batch {
                            let emitted =
                                catch_unwind(AssertUnwindSafe(|| process.on_event(event)))
                                    .map_err(|_| RuntimeError::HandlerPanicked(lp_id))?;
                            outbound.extend(emitted.into_iter().map(|out| (event.tick, out)));
                        }
                        Ok::<_, RuntimeError>(outbound)
                    });
                match spawn {
                    Ok(handle) => handles.push((lp_id, handle)),
                    Err(_) => spawn_error = Some(RuntimeError::SpawnFailed(lp_id)),
                }
            }
            let outcomes = handles
                .into_iter()
                .map(|(lp_id, handle)| {
                    let result = handle
                        .join()
                        .unwrap_or(Err(RuntimeError::HandlerPanicked(lp_id)));
                    (lp_id, result)
                })
                .collect::<Vec<_>>();
            (outcomes, spawn_error)
        });

        if let Some(error) = spawn_error {
            self.poisoned = true;
            return Err(error);
        }

        let mut staged = Vec::new();
        for (lp_id, result) in outcomes {
            let outbound = match result {
                Ok(outbound) => outbound,
                Err(error) => {
                    self.poisoned = true;
                    return Err(error);
                }
            };
            for (event_tick, event) in outbound {
                if event.source_lp != lp_id {
                    self.poisoned = true;
                    return Err(RuntimeError::OutboundSourceMismatch {
                        emitting_lp: lp_id,
                        declared_source: event.source_lp,
                    });
                }
                if !self.processes.contains_key(&event.dest_lp) {
                    self.poisoned = true;
                    return Err(RuntimeError::UnknownOutboundDestination(event.dest_lp));
                }
                if !self.has_route(lp_id, event.dest_lp) {
                    self.poisoned = true;
                    return Err(RuntimeError::OutboundRouteMissing {
                        source: lp_id,
                        destination: event.dest_lp,
                    });
                }
                let minimum_tick = if event.dest_lp == lp_id {
                    event_tick
                } else {
                    event_tick.checked_add(lookahead).ok_or_else(|| {
                        self.poisoned = true;
                        RuntimeError::LookaheadOverflow {
                            lp_id,
                            event_tick,
                            lookahead,
                        }
                    })?
                };
                if event.tick < minimum_tick {
                    self.poisoned = true;
                    return Err(RuntimeError::OutboundLookaheadViolation {
                        lp_id,
                        event_tick,
                        outbound_tick: event.tick,
                        minimum_tick,
                    });
                }
                let local_time = self.local_horizons[&event.dest_lp];
                if event.tick < local_time {
                    self.poisoned = true;
                    return Err(RuntimeError::Straggler {
                        lp_id: event.dest_lp,
                        event_tick: event.tick,
                        local_time,
                    });
                }
                staged.push(event);
            }
        }

        // Commit all messages only after validating the complete round.
        let emitted_count =
            u64::try_from(staged.len()).map_err(|_| RuntimeError::EventSequenceOverflow)?;
        let remote_count = u64::try_from(
            staged
                .iter()
                .filter(|event| event.dest_lp != event.source_lp)
                .count(),
        )
        .map_err(|_| RuntimeError::EventSequenceOverflow)?;
        if self.next_sequence.checked_add(emitted_count).is_none() {
            self.poisoned = true;
            return Err(RuntimeError::EventSequenceOverflow);
        }
        for event in staged {
            if let Err(error) = self.enqueue(event) {
                self.poisoned = true;
                return Err(error);
            }
        }
        self.report.processed_events = self.report.processed_events.saturating_add(
            batches
                .values()
                .map(|batch| batch.len() as u64)
                .sum::<u64>(),
        );
        self.report.remote_events = self.report.remote_events.saturating_add(remote_count);
        self.report.emitted_events = self.report.emitted_events.saturating_add(emitted_count);
        self.report.worker_count = self
            .report
            .worker_count
            .max(batches.values().filter(|batch| !batch.is_empty()).count());
        Ok(())
    }

    fn advance_bounds(&mut self, safe: &BTreeMap<LpId, Tick>, horizon: Tick) -> bool {
        let mut advanced = false;
        for (&lp_id, &frontier) in safe {
            let pending = self.queues[&lp_id]
                .keys()
                .next()
                .map(|(tick, _, _)| *tick)
                .unwrap_or(horizon);
            let frontier = frontier.min(pending);
            let local = self
                .local_horizons
                .get_mut(&lp_id)
                .expect("known LP frontier");
            if frontier > *local {
                *local = frontier;
                advanced = true;
            }
        }

        let lookahead = self.partition.lookahead();
        let mut updates = Vec::new();
        for (&source, destinations) in &self.topology {
            let pending = self.queues[&source]
                .keys()
                .next()
                .map(|(tick, _, _)| *tick)
                .unwrap_or(horizon);
            let input_frontier = safe[&source].min(pending);
            let advanced_bound = input_frontier
                .checked_add(lookahead)
                .unwrap_or(horizon)
                .min(horizon);
            for &destination in destinations {
                let key = (source, destination);
                if advanced_bound > self.outbound_bounds[&key] {
                    updates.push((key, advanced_bound));
                }
            }
        }
        for (key, bound) in updates {
            self.outbound_bounds.insert(key, bound);
            self.report.null_messages = self.report.null_messages.saturating_add(1);
            advanced = true;
        }
        advanced
    }

    fn refresh_gvt(&mut self, safe: &BTreeMap<LpId, Tick>, inclusive_horizon: Tick) {
        for (&lp_id, &frontier) in safe {
            let pending = self.queues[&lp_id]
                .keys()
                .next()
                .map(|(tick, _, _)| *tick)
                .unwrap_or(frontier);
            let local = self
                .local_horizons
                .get_mut(&lp_id)
                .expect("known LP frontier");
            *local = (*local).max(frontier.min(pending));
        }
        let queued_min = self
            .queues
            .values()
            .flat_map(|queue| queue.keys().map(|(tick, _, _)| *tick))
            .min();
        let horizon_min = self
            .local_horizons
            .values()
            .copied()
            .map(|exclusive| {
                exclusive
                    .checked_sub(SimDuration::from_ticks(1))
                    .unwrap_or(SimTime::ZERO)
            })
            .min()
            .unwrap_or(self.gvt);
        let candidate = queued_min.map_or(horizon_min, |tick| tick.min(horizon_min));
        let next = self.gvt.max(candidate.min(inclusive_horizon));
        if next != self.gvt {
            self.gvt = next;
            self.report.gvt_history.push(next);
        }
    }

    fn is_complete(&self, horizon: Tick) -> bool {
        self.local_horizons
            .values()
            .all(|frontier| *frontier >= horizon)
            && self.queues.values().all(|queue| {
                queue
                    .keys()
                    .next()
                    .map_or(true, |(tick, _, _)| *tick >= horizon)
            })
    }
}

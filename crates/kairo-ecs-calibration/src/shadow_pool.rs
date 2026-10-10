//! Bounded, isolated C3 probe runtime ownership and typed checkpoints.
use crate::residuals::LogicalKey;
use crate::shadow::{
    LedgerSnapshot, ProbeAdapter, ProbeBudget, ProbeCheckpoint, ProbeInput, ProbeOutcome,
    ProbeResult, ProbeSpec, RunnerCheckpoint, SavedProbeState, ShadowError,
};
use crate::trace_order::TraceOrderKeyV1;
use std::collections::BTreeMap;
use std::mem::size_of;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PoolLimits {
    pub max_probes: usize,
    pub max_snapshot_bytes: usize,
    pub max_probe_image_bytes: usize,
    pub max_checkpoint_bytes: usize,
}

struct ProbeWorld<R> {
    spec: ProbeSpec,
    snapshot: LedgerSnapshot,
    runtime: Option<R>,
    events: u64,
    last_tick: u128,
    terminal: Option<ProbeOutcome>,
}

/// Owns the adapter and each admitted mutable world separately.
pub(crate) struct ProbePool<A: ProbeAdapter> {
    adapter: A,
    binding: [u8; 32],
    limits: PoolLimits,
    probes: BTreeMap<String, ProbeWorld<A::Runtime>>,
}

impl<A: ProbeAdapter> ProbePool<A> {
    pub(crate) fn new(adapter: A, binding: [u8; 32], limits: PoolLimits) -> Self {
        Self {
            adapter,
            binding,
            limits,
            probes: BTreeMap::new(),
        }
    }

    pub(crate) fn admit(
        &mut self,
        spec: ProbeSpec,
        snapshot: LedgerSnapshot,
    ) -> Result<(), ShadowError> {
        validate_binding(&spec, &snapshot)?;
        if self.probes.contains_key(&spec.id) {
            return Err(ShadowError::DuplicateIdentity(spec.id));
        }
        let snapshot_bytes = snapshot_size(&snapshot)?;
        let spec_bytes = spec_size(&spec)?;
        if self.probes.len() >= self.limits.max_probes
            || snapshot_bytes > self.limits.max_snapshot_bytes
            || checked_add(spec_bytes, snapshot_bytes)? > self.limits.max_checkpoint_bytes
        {
            return Err(ShadowError::LimitExceeded);
        }
        let mut retained_bytes = checked_add(
            checked_add(spec_bytes, snapshot_bytes)?,
            size_of::<ProbeCheckpoint>() + 3 * size_of::<usize>(),
        )?;
        for existing in self.probes.values() {
            retained_bytes = checked_add(retained_bytes, spec_size(&existing.spec)?)?;
            retained_bytes = checked_add(retained_bytes, snapshot_size(&existing.snapshot)?)?;
            retained_bytes = checked_add(
                retained_bytes,
                size_of::<ProbeCheckpoint>() + 3 * size_of::<usize>(),
            )?;
        }
        if retained_bytes > self.limits.max_checkpoint_bytes {
            return Err(ShadowError::LimitExceeded);
        }
        if spec.budget.horizon < snapshot.at {
            return Err(ShadowError::InvalidInput("horizon precedes anchor"));
        }
        let infeasible = !snapshot.resource_feasible;
        let runtime = if infeasible {
            None
        } else {
            Some(self.adapter.start(&snapshot, &spec.input)?)
        };
        if runtime
            .as_ref()
            .is_some_and(|r| self.adapter.now(r) != snapshot.at)
        {
            return Err(ShadowError::Contract(
                "runtime does not start at snapshot tick",
            ));
        }
        let mut world = ProbeWorld {
            spec,
            snapshot,
            runtime,
            events: 0,
            last_tick: 0,
            terminal: infeasible.then(|| ProbeOutcome::Infeasible {
                reason: "resource snapshot is infeasible".into(),
            }),
        };
        world.last_tick = world.snapshot.at;
        if let Some(runtime) = world.runtime.as_ref() {
            match self.adapter.target_at_start(runtime)? {
                Some(tick) if tick == world.snapshot.at => {
                    world.terminal = Some(ProbeOutcome::Completed { predicted: tick });
                    world.runtime = None;
                }
                Some(_) => {
                    return Err(ShadowError::Contract(
                        "admission target is not at current tick",
                    ))
                }
                None => {}
            }
        }
        let id = world.spec.id.clone();
        self.probes.insert(id, world);
        Ok(())
    }

    /// Advance at most `max_dispatches` this call, in addition to lifetime limits.
    /// Returns `None` while pending and the same terminal result on every later poll.
    pub(crate) fn advance(
        &mut self,
        id: &str,
        max_dispatches: u64,
    ) -> Result<Option<ProbeResult>, ShadowError> {
        let world = self
            .probes
            .get_mut(id)
            .ok_or_else(|| ShadowError::UnknownProbe(id.to_owned()))?;
        if world.terminal.is_some() {
            return Ok(Some(result(world)));
        }
        let mut local = 0;
        while local < max_dispatches {
            let runtime = world.runtime.as_mut().expect("pending probe owns runtime");
            let now = self.adapter.now(runtime);
            if now != world.last_tick {
                return Err(poison_contract(
                    world,
                    "runtime tick changed outside dispatch",
                ));
            }
            let next = match self.adapter.next_tick(runtime) {
                Ok(v) => v,
                Err(e) => {
                    finish(
                        world,
                        ProbeOutcome::Failed {
                            reason: format!("{e:?}"),
                        },
                    );
                    return Ok(Some(result(world)));
                }
            };
            let Some(next_tick) = next else {
                finish(world, ProbeOutcome::Missing);
                return Ok(Some(result(world)));
            };
            if next_tick < now {
                return Err(poison_contract(
                    world,
                    "next dispatch precedes runtime time",
                ));
            }
            if next_tick > world.spec.budget.horizon {
                finish(
                    world,
                    ProbeOutcome::Censored {
                        reason: crate::shadow::LimitReason::TickHorizon,
                    },
                );
                return Ok(Some(result(world)));
            }
            if world.events >= world.spec.budget.max_events {
                finish(
                    world,
                    ProbeOutcome::Censored {
                        reason: crate::shadow::LimitReason::EventBudget,
                    },
                );
                return Ok(Some(result(world)));
            }
            let step = match self.adapter.step(runtime) {
                Ok(step) => step,
                Err(e) => {
                    finish(
                        world,
                        ProbeOutcome::Failed {
                            reason: format!("{e:?}"),
                        },
                    );
                    return Ok(Some(result(world)));
                }
            };
            world.events = world
                .events
                .checked_add(1)
                .ok_or(ShadowError::LimitExceeded)?;
            local += 1;
            if step.dispatched_at != next_tick || self.adapter.now(runtime) != step.dispatched_at {
                return Err(poison_contract(
                    world,
                    "dispatch receipt or runtime tick mismatch",
                ));
            }
            if step.dispatched_at < world.last_tick {
                return Err(poison_contract(world, "runtime time reversal"));
            }
            world.last_tick = step.dispatched_at;
            if let Some(target) = step.target {
                if target != step.dispatched_at {
                    return Err(poison_contract(world, "target outside dispatched prefix"));
                }
                finish(world, ProbeOutcome::Completed { predicted: target });
                return Ok(Some(result(world)));
            }
        }
        Ok(None)
    }

    pub(crate) fn results(&self) -> Vec<ProbeResult> {
        self.probes
            .values()
            .filter(|p| p.terminal.is_some())
            .map(result)
            .collect()
    }

    pub(crate) fn checkpoint(&self) -> Result<RunnerCheckpoint, ShadowError> {
        if self.probes.len() > self.limits.max_probes {
            return Err(ShadowError::LimitExceeded);
        }
        let vector_bytes = self
            .probes
            .len()
            .checked_mul(size_of::<ProbeCheckpoint>())
            .ok_or(ShadowError::LimitExceeded)?;
        let mut bytes = checked_add(size_of::<RunnerCheckpoint>(), vector_bytes)?;
        for world in self.probes.values() {
            bytes = checked_add(bytes, spec_size(&world.spec)?)?;
            bytes = checked_add(bytes, snapshot_size(&world.snapshot)?)?;
            if let Some(outcome) = &world.terminal {
                bytes = checked_add(bytes, outcome_size(outcome)?)?;
            }
        }
        if bytes > self.limits.max_checkpoint_bytes {
            return Err(ShadowError::LimitExceeded);
        }
        let mut probes = Vec::with_capacity(self.probes.len());
        for world in self.probes.values() {
            let state = if let Some(outcome) = &world.terminal {
                SavedProbeState::Terminal(outcome.clone())
            } else {
                let remaining = self
                    .limits
                    .max_checkpoint_bytes
                    .checked_sub(bytes)
                    .ok_or(ShadowError::LimitExceeded)?;
                let image_cap = remaining.min(self.limits.max_probe_image_bytes);
                if image_cap == 0 {
                    return Err(ShadowError::LimitExceeded);
                }
                let image = self
                    .adapter
                    .checkpoint(world.runtime.as_ref().expect("pending runtime"), image_cap)?;
                if image.capacity() > image_cap {
                    return Err(ShadowError::LimitExceeded);
                }
                bytes = checked_add(bytes, image.capacity())?;
                SavedProbeState::Pending(image)
            };
            if bytes > self.limits.max_checkpoint_bytes {
                return Err(ShadowError::LimitExceeded);
            }
            probes.push(ProbeCheckpoint {
                spec: world.spec.clone(),
                snapshot: world.snapshot.clone(),
                events: world.events,
                last_tick: world.last_tick,
                state,
            });
        }
        if bytes > self.limits.max_checkpoint_bytes {
            return Err(ShadowError::LimitExceeded);
        }
        Ok(RunnerCheckpoint {
            version: 1,
            binding: self.binding,
            probes,
        })
    }

    /// Verify the complete trusted inventory and all bounds before decoding any runtime.
    pub(crate) fn restore(
        adapter: A,
        binding: [u8; 32],
        limits: PoolLimits,
        trusted: Vec<(ProbeSpec, LedgerSnapshot)>,
        checkpoint: RunnerCheckpoint,
    ) -> Result<Self, ShadowError> {
        if checkpoint.version != 1
            || checkpoint.binding != binding
            || trusted.len() > limits.max_probes
            || checkpoint.probes.len() != trusted.len()
        {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        let mut trusted_map = BTreeMap::new();
        for (spec, snapshot) in trusted {
            if trusted_map
                .insert(spec.id.clone(), (spec, snapshot))
                .is_some()
            {
                return Err(ShadowError::IncompatibleCheckpoint);
            }
        }
        let mut saved_map = BTreeMap::new();
        for saved in checkpoint.probes {
            if saved_map.insert(saved.spec.id.clone(), saved).is_some() {
                return Err(ShadowError::IncompatibleCheckpoint);
            }
        }
        let mut byte_count = checked_add(
            size_of::<RunnerCheckpoint>(),
            saved_map
                .len()
                .checked_mul(size_of::<ProbeCheckpoint>())
                .ok_or(ShadowError::LimitExceeded)?,
        )?;
        for (id, (spec, snapshot)) in &trusted_map {
            let saved = saved_map
                .get(id)
                .ok_or(ShadowError::IncompatibleCheckpoint)?;
            if &saved.spec != spec
                || &saved.snapshot != snapshot
                || validate_binding(spec, snapshot).is_err()
                || spec.budget.horizon < snapshot.at
                || saved.events > spec.budget.max_events
                || saved.last_tick < snapshot.at
                || saved.last_tick > spec.budget.horizon
                || snapshot_size(snapshot)? > limits.max_snapshot_bytes
            {
                return Err(ShadowError::IncompatibleCheckpoint);
            }
            byte_count = checked_add(byte_count, snapshot_size(snapshot)?)?;
            byte_count = checked_add(byte_count, spec_size(spec)?)?;
            match &saved.state {
                SavedProbeState::Pending(image) => {
                    if !snapshot.resource_feasible
                        || image.capacity() > limits.max_probe_image_bytes
                    {
                        return Err(ShadowError::IncompatibleCheckpoint);
                    }
                    byte_count = checked_add(byte_count, image.capacity())?;
                }
                SavedProbeState::Terminal(outcome) => {
                    if !terminal_valid(
                        outcome,
                        saved.events,
                        saved.last_tick,
                        snapshot,
                        spec.budget,
                    ) {
                        return Err(ShadowError::IncompatibleCheckpoint);
                    }
                    byte_count = checked_add(byte_count, outcome_size(outcome)?)?;
                }
            }
        }
        if byte_count > limits.max_checkpoint_bytes {
            return Err(ShadowError::LimitExceeded);
        }
        // Decode only after all checkpoint metadata and trusted inventory match.
        let mut pool = Self::new(adapter, binding, limits);
        for (id, (spec, snapshot)) in trusted_map {
            let saved = saved_map
                .remove(&id)
                .ok_or(ShadowError::IncompatibleCheckpoint)?;
            let (runtime, terminal) = match saved.state {
                SavedProbeState::Terminal(outcome) => (None, Some(outcome)),
                SavedProbeState::Pending(image) => {
                    let runtime = pool.adapter.restore(&snapshot, &spec.input, &image)?;
                    if pool.adapter.now(&runtime) != saved.last_tick
                        || pool.adapter.target_at_start(&runtime)?.is_some()
                    {
                        return Err(ShadowError::IncompatibleCheckpoint);
                    }
                    (Some(runtime), None)
                }
            };
            pool.probes.insert(
                id,
                ProbeWorld {
                    spec,
                    snapshot,
                    runtime,
                    events: saved.events,
                    last_tick: saved.last_tick,
                    terminal,
                },
            );
        }
        Ok(pool)
    }
}

fn finish<R>(world: &mut ProbeWorld<R>, outcome: ProbeOutcome) {
    world.runtime = None;
    world.terminal = Some(outcome);
}

fn result<R>(world: &ProbeWorld<R>) -> ProbeResult {
    ProbeResult {
        id: world.spec.id.clone(),
        anchor_event: world.spec.anchor_event.clone(),
        frontier: world.snapshot.frontier,
        snapshot_digest: world.snapshot.digest,
        observed: world.spec.observed_target,
        outcome: world.terminal.clone().expect("terminal result"),
        events: world.events,
        last_tick: world.last_tick,
        assumptions: world.snapshot.assumptions.clone(),
    }
}

fn terminal_valid(
    outcome: &ProbeOutcome,
    events: u64,
    last: u128,
    snapshot: &LedgerSnapshot,
    budget: ProbeBudget,
) -> bool {
    match outcome {
        ProbeOutcome::Completed { predicted } => {
            snapshot.resource_feasible
                && *predicted == last
                && *predicted >= snapshot.at
                && *predicted <= budget.horizon
        }
        ProbeOutcome::Infeasible { .. } => {
            !snapshot.resource_feasible && events == 0 && last == snapshot.at
        }
        ProbeOutcome::Missing | ProbeOutcome::Failed { .. } => snapshot.resource_feasible,
        ProbeOutcome::Censored {
            reason: crate::shadow::LimitReason::EventBudget,
        } => snapshot.resource_feasible && events == budget.max_events,
        ProbeOutcome::Censored {
            reason: crate::shadow::LimitReason::TickHorizon,
        } => snapshot.resource_feasible && last <= budget.horizon,
    }
}

fn poison_contract<R>(world: &mut ProbeWorld<R>, message: &'static str) -> ShadowError {
    finish(
        world,
        ProbeOutcome::Failed {
            reason: format!("adapter contract violation: {message}"),
        },
    );
    ShadowError::Contract(message)
}

fn validate_binding(spec: &ProbeSpec, snapshot: &LedgerSnapshot) -> Result<(), ShadowError> {
    let required = [
        spec.id.as_str(),
        spec.run_id.as_str(),
        spec.candidate_id.as_str(),
        spec.anchor_event.as_str(),
        spec.input.target.as_str(),
        spec.input.fidelity.as_str(),
        spec.key.study_id.as_str(),
        spec.key.dataset_id.as_str(),
        spec.key.scenario_id.as_str(),
        spec.key.seed_schedule_id.as_str(),
        spec.key.replication_id.as_str(),
        spec.key.case_key.as_str(),
        spec.key.task_key.as_str(),
        spec.key.endpoint.as_str(),
        spec.key.seed_purpose.as_str(),
        spec.key.seed_map_ref.as_str(),
        spec.key.mapping_version.as_str(),
    ];
    if required.iter().any(|value| value.trim().is_empty())
        || spec.id != spec.id.trim()
        || !matches!(spec.input.fidelity.as_str(), "Macro" | "Micro")
        || spec.input.target != spec.key.endpoint
        || spec.anchor_event != snapshot.anchor_event
        || spec.key.seed_purpose.as_str() != "service"
            && spec.key.seed_purpose.as_str() != "transit"
            && spec.key.seed_purpose.as_str() != "behavior"
            && spec.key.seed_purpose.as_str() != "calibration"
        || spec
            .target_event
            .as_ref()
            .is_some_and(|event| event.trim().is_empty())
    {
        return Err(ShadowError::InvalidInput("probe provenance or anchor"));
    }
    let anchor_count = snapshot
        .visible_events
        .iter()
        .filter(|event| event.order.source_event_key == snapshot.anchor_event)
        .count();
    let anchor = snapshot
        .visible_events
        .iter()
        .find(|event| event.order.source_event_key == snapshot.anchor_event);
    if anchor_count != 1
        || !anchor.is_some_and(|event| {
            event.source_defined && event.available_at.is_some_and(|tick| tick <= snapshot.at)
        })
    {
        return Err(ShadowError::UnavailableAnchor(
            snapshot.anchor_event.clone(),
        ));
    }
    Ok(())
}

fn spec_size(spec: &ProbeSpec) -> Result<usize, ShadowError> {
    let strings = [
        &spec.id,
        &spec.run_id,
        &spec.candidate_id,
        &spec.anchor_event,
        &spec.input.target,
        &spec.input.fidelity,
        &spec.key.study_id,
        &spec.key.dataset_id,
        &spec.key.scenario_id,
        &spec.key.seed_schedule_id,
        &spec.key.replication_id,
        &spec.key.case_key,
        &spec.key.task_key,
        &spec.key.endpoint,
        &spec.key.seed_purpose,
        &spec.key.seed_map_ref,
        &spec.key.mapping_version,
    ];
    let string_bytes = checked_sum(strings.iter().map(|s| s.capacity()))?;
    let seed_bytes = spec
        .input
        .seed_key
        .checkpoint_identifier_bytes()
        .map_err(|_| ShadowError::LimitExceeded)?;
    let target_bytes = spec.target_event.as_ref().map_or(0, String::capacity);
    let fixed = checked_sum(
        [
            size_of::<ProbeSpec>(),
            size_of::<LogicalKey>(),
            size_of::<ProbeInput>(),
        ]
        .into_iter(),
    )?;
    checked_add(
        fixed,
        checked_add(string_bytes, checked_add(seed_bytes, target_bytes)?)?,
    )
}

fn snapshot_size(snapshot: &LedgerSnapshot) -> Result<usize, ShadowError> {
    let assumptions = checked_add(
        snapshot
            .assumptions
            .capacity()
            .checked_mul(size_of::<String>())
            .ok_or(ShadowError::LimitExceeded)?,
        checked_sum(snapshot.assumptions.iter().map(String::capacity))?,
    )?;
    let visible = snapshot.visible_events.iter().try_fold(
        snapshot
            .visible_events
            .capacity()
            .checked_mul(size_of::<crate::shadow::ObservedEvent>())
            .ok_or(ShadowError::LimitExceeded)?,
        |total, event| {
            let event_size = checked_sum(
                [
                    size_of::<crate::shadow::ObservedEvent>(),
                    size_of::<TraceOrderKeyV1>(),
                    size_of::<crate::shadow::Transition>(),
                    event.payload.capacity(),
                    event.order.case_key.capacity(),
                    event.order.event_kind_rank.as_str().len(),
                    event.order.source_event_key.capacity(),
                    transition_bytes(&event.transition)?,
                ]
                .into_iter(),
            )?;
            total
                .checked_add(event_size)
                .ok_or(ShadowError::LimitExceeded)
        },
    )?;
    let mut n = size_of::<LedgerSnapshot>()
        .checked_add(
            snapshot
                .visible_events
                .capacity()
                .checked_mul(3 * size_of::<usize>())
                .ok_or(ShadowError::LimitExceeded)?,
        )
        .and_then(|n| {
            n.checked_add(snapshot.resources.len().checked_mul(
                size_of::<(String, crate::shadow::ResourceState)>() + 3 * size_of::<usize>(),
            )?)
        })
        .and_then(|n| n.checked_add(snapshot.anchor_event.capacity()))
        .and_then(|v| v.checked_add(assumptions))
        .and_then(|v| v.checked_add(visible))
        .ok_or(ShadowError::LimitExceeded)?;
    for (name, resource) in &snapshot.resources {
        let claims = checked_add(
            resource
                .claims
                .len()
                .checked_mul(size_of::<(String, u32)>() + 3 * size_of::<usize>())
                .ok_or(ShadowError::LimitExceeded)?,
            checked_sum(resource.claims.keys().map(String::capacity))?,
        )?;
        n = n
            .checked_add(name.capacity())
            .and_then(|v| v.checked_add(claims))
            .ok_or(ShadowError::LimitExceeded)?;
    }
    Ok(n)
}

fn transition_bytes(transition: &crate::shadow::Transition) -> Result<usize, ShadowError> {
    match transition {
        crate::shadow::Transition::None => Ok(0),
        crate::shadow::Transition::Acquire {
            resource, claim, ..
        } => checked_add(resource.capacity(), claim.capacity()),
        crate::shadow::Transition::Release { resource, claim } => {
            checked_add(resource.capacity(), claim.capacity())
        }
    }
}

fn outcome_size(outcome: &ProbeOutcome) -> Result<usize, ShadowError> {
    let reason = match outcome {
        ProbeOutcome::Infeasible { reason } | ProbeOutcome::Failed { reason } => reason.capacity(),
        _ => 0,
    };
    checked_add(size_of::<ProbeOutcome>(), reason)
}

fn checked_add(left: usize, right: usize) -> Result<usize, ShadowError> {
    left.checked_add(right).ok_or(ShadowError::LimitExceeded)
}

fn checked_sum(mut values: impl Iterator<Item = usize>) -> Result<usize, ShadowError> {
    values.try_fold(0usize, |total, value| {
        total.checked_add(value).ok_or(ShadowError::LimitExceeded)
    })
}

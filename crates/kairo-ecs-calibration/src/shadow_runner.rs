//! Trusted observed replay and isolated probe orchestration for C3.
use crate::shadow::{
    Evaluation, EvaluationPolicy, LedgerDiagnostic, LedgerSnapshot, ObservedEvent, ProbeAdapter,
    ProbeSpec, ResourcePolicy, ShadowError,
};
use crate::shadow_ledger::{InitialResource, LedgerLimits, ObservedLedger};
use crate::shadow_pool::{PoolLimits, ProbePool};
use crate::shadow_report;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Immutable source/configuration supplied by the trusted model integration.
pub(crate) struct TrustedRunDefinition {
    pub identity: Vec<u8>,
    pub events: Vec<ObservedEvent>,
    pub initial_resources: BTreeMap<String, InitialResource>,
    pub assumptions: Vec<String>,
    pub resource_policy: ResourcePolicy,
    pub ledger_limits: LedgerLimits,
    /// Target/observed fields are checked against, then derived from, source events.
    pub probes: Vec<ProbeSpec>,
    pub pool_limits: PoolLimits,
}

type TrustedInventory = Vec<(ProbeSpec, LedgerSnapshot)>;
type ProbePlans = BTreeMap<String, Vec<ProbeSpec>>;

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ShadowRunnerCheckpoint {
    pub ledger_frontier: usize,
    pub pool: crate::shadow::RunnerCheckpoint,
}

pub(crate) struct ShadowRunner<A: ProbeAdapter> {
    ledger: ObservedLedger,
    pool: ProbePool<A>,
    plans: BTreeMap<String, Vec<ProbeSpec>>,
    admitted: BTreeSet<String>,
    skipped: BTreeMap<String, String>,
    source_event_count: usize,
    planned_probe_count: usize,
    fault: Option<ShadowError>,
    inventory: BTreeMap<String, (ProbeSpec, LedgerSnapshot)>,
}

impl<A: ProbeAdapter> ShadowRunner<A> {
    pub(crate) fn new(
        mut definition: TrustedRunDefinition,
        adapter: A,
    ) -> Result<Self, ShadowError> {
        let (binding, plans) = prepare(&mut definition)?;
        let source_event_count = definition.events.len();
        let planned_probe_count = definition.probes.len();
        let ledger = ObservedLedger::new(
            definition.events,
            definition.initial_resources,
            definition.assumptions,
            definition.resource_policy,
            definition.ledger_limits,
        )?;
        Ok(Self {
            ledger,
            pool: ProbePool::new(adapter, binding, definition.pool_limits),
            plans,
            admitted: BTreeSet::new(),
            skipped: BTreeMap::new(),
            source_event_count,
            planned_probe_count,
            fault: None,
            inventory: BTreeMap::new(),
        })
    }

    /// Advance one trusted source occurrence. Unavailable anchors still advance
    /// the ledger frontier but never admit a probe.
    pub(crate) fn advance_source(
        &mut self,
        max_dispatches_per_probe: u64,
    ) -> Result<bool, ShadowError> {
        self.ensure_healthy()?;
        let before = self.ledger.frontier();
        let mut unavailable = None;
        let advance = match self.ledger.advance() {
            Ok(Some(snapshot)) => Some(snapshot.clone()),
            Ok(None) => None,
            Err(ShadowError::UnavailableAnchor(key)) => {
                unavailable = Some(key);
                None
            }
            Err(error) => {
                self.fault = Some(error.clone());
                return Err(error);
            }
        };
        if let Some(key) = unavailable {
            if let Some(specs) = self.plans.get(&key) {
                for spec in specs {
                    self.skipped
                        .insert(spec.id.clone(), "unavailable anchor".into());
                }
            }
        }
        if let Some(snapshot) = advance {
            if let Some(specs) = self.plans.get(&snapshot.anchor_event) {
                for spec in specs {
                    if !self.admitted.contains(&spec.id) {
                        if let Err(error) = self.pool.admit(spec.clone(), snapshot.clone()) {
                            self.fault = Some(error.clone());
                            return Err(error);
                        }
                        self.inventory
                            .insert(spec.id.clone(), (spec.clone(), snapshot.clone()));
                        self.admitted.insert(spec.id.clone());
                    }
                }
            }
        }
        self.drive_probes(max_dispatches_per_probe)?;
        Ok(self.ledger.frontier() > before)
    }

    /// Drive pending probes in canonical stable-ID order without moving source time.
    pub(crate) fn drive_probes(
        &mut self,
        max_dispatches_per_probe: u64,
    ) -> Result<(), ShadowError> {
        self.ensure_healthy()?;
        let ids: Vec<_> = self.admitted.iter().cloned().collect();
        for id in ids {
            if let Err(error) = self.pool.advance(&id, max_dispatches_per_probe) {
                self.fault = Some(error.clone());
                return Err(error);
            }
        }
        Ok(())
    }

    pub(crate) fn results(&self) -> Vec<crate::shadow::ProbeResult> {
        self.pool.results()
    }

    pub(crate) fn frontier(&self) -> usize {
        self.ledger.frontier()
    }

    pub(crate) fn diagnostics(&self) -> &[LedgerDiagnostic] {
        self.ledger.diagnostics()
    }

    pub(crate) fn historical_feasible(&self) -> bool {
        self.ledger.diagnostics().is_empty()
    }

    pub(crate) fn evaluate(&self, policy: EvaluationPolicy) -> Result<Evaluation, ShadowError> {
        let results = self.pool.results();
        let terminal_ids: BTreeSet<_> = results.iter().map(|result| result.id.as_str()).collect();
        let omitted = self.planned_probe_count.saturating_sub(terminal_ids.len());
        let mut evaluation = shadow_report::evaluate(&results, policy)?;
        evaluation.counts.total = self.planned_probe_count as u64;
        evaluation.counts.missing_prediction = evaluation
            .counts
            .missing_prediction
            .saturating_add(omitted as u64);
        let complete = self.ledger.frontier() == self.source_event_count
            && self.admitted.len() + self.skipped.len() == self.planned_probe_count
            && results.len() == self.admitted.len();
        if matches!(policy, EvaluationPolicy::Strict { .. })
            && (self.fault.is_some()
                || !self.historical_feasible()
                || !complete
                || !self.skipped.is_empty())
        {
            evaluation.accepted = false;
        }
        Ok(evaluation)
    }

    pub(crate) fn skipped_probes(&self) -> &BTreeMap<String, String> {
        &self.skipped
    }

    /// Trusted immutable inventory for portable checkpoint framing; no runtimes are started.
    pub(crate) fn trusted_inventory(&self) -> Vec<(ProbeSpec, LedgerSnapshot)> {
        self.inventory.values().cloned().collect()
    }

    fn ensure_healthy(&self) -> Result<(), ShadowError> {
        match &self.fault {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    pub(crate) fn checkpoint(&self) -> Result<ShadowRunnerCheckpoint, ShadowError> {
        self.ensure_healthy()?;
        Ok(ShadowRunnerCheckpoint {
            ledger_frontier: self.ledger.frontier(),
            pool: self.pool.checkpoint()?,
        })
    }

    /// Reconstruct source snapshots without starting prediction runtimes, then
    /// validate the complete admitted inventory before pool runtime decoding.
    pub(crate) fn restore(
        mut definition: TrustedRunDefinition,
        adapter: A,
        checkpoint: ShadowRunnerCheckpoint,
    ) -> Result<Self, ShadowError> {
        let (binding, plans) = prepare(&mut definition)?;
        let source_event_count = definition.events.len();
        let planned_probe_count = definition.probes.len();
        if checkpoint.ledger_frontier > definition.events.len()
            || checkpoint.pool.binding != binding
        {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        let mut ledger = ObservedLedger::new(
            definition.events,
            definition.initial_resources,
            definition.assumptions,
            definition.resource_policy,
            definition.ledger_limits,
        )?;
        let mut trusted = BTreeMap::<String, (ProbeSpec, LedgerSnapshot)>::new();
        let mut admitted = BTreeSet::new();
        let mut skipped = BTreeMap::new();
        for _ in 0..checkpoint.ledger_frontier {
            let mut unavailable = None;
            let snapshot = match ledger.advance() {
                Ok(Some(snapshot)) => Some(snapshot.clone()),
                Ok(None) => None,
                Err(ShadowError::UnavailableAnchor(key)) => {
                    unavailable = Some(key);
                    None
                }
                Err(error) => return Err(error),
            };
            if let Some(key) = unavailable {
                if let Some(specs) = plans.get(&key) {
                    for spec in specs {
                        skipped.insert(spec.id.clone(), "unavailable anchor".into());
                    }
                }
            }
            if let Some(snapshot) = snapshot {
                if let Some(specs) = plans.get(&snapshot.anchor_event) {
                    for spec in specs {
                        admitted.insert(spec.id.clone());
                        if trusted
                            .insert(spec.id.clone(), (spec.clone(), snapshot.clone()))
                            .is_some()
                        {
                            return Err(ShadowError::DuplicateIdentity(spec.id.clone()));
                        }
                    }
                }
            }
        }
        let trusted_inventory = trusted.clone();
        let restored = ProbePool::restore(
            adapter,
            binding,
            definition.pool_limits,
            trusted.into_values().collect(),
            checkpoint.pool,
        )?;
        Ok(Self {
            ledger,
            pool: restored,
            plans,
            admitted,
            skipped,
            source_event_count,
            planned_probe_count,
            fault: None,
            inventory: trusted_inventory,
        })
    }
}

/// Reconstruct the exact trusted pool inventory through a portable checkpoint frontier.
/// This function owns no adapter and cannot start or restore a prediction runtime.
pub(crate) fn trusted_inventory(
    definition: &mut TrustedRunDefinition,
    frontier: usize,
) -> Result<([u8; 32], TrustedInventory), ShadowError> {
    let (binding, plans) = prepare(definition)?;
    if frontier > definition.events.len() {
        return Err(ShadowError::IncompatibleCheckpoint);
    }
    let mut ledger = ObservedLedger::new(
        definition.events.clone(),
        definition.initial_resources.clone(),
        definition.assumptions.clone(),
        definition.resource_policy,
        definition.ledger_limits,
    )?;
    let mut inventory = BTreeMap::<String, (ProbeSpec, LedgerSnapshot)>::new();
    for _ in 0..frontier {
        let snapshot = match ledger.advance() {
            Ok(Some(snapshot)) => Some(snapshot.clone()),
            Ok(None) | Err(ShadowError::UnavailableAnchor(_)) => None,
            Err(error) => return Err(error),
        };
        if let Some(snapshot) = snapshot {
            if let Some(specs) = plans.get(&snapshot.anchor_event) {
                for spec in specs {
                    if inventory
                        .insert(spec.id.clone(), (spec.clone(), snapshot.clone()))
                        .is_some()
                    {
                        return Err(ShadowError::DuplicateIdentity(spec.id.clone()));
                    }
                }
            }
        }
    }
    Ok((binding, inventory.into_values().collect()))
}

fn prepare(definition: &mut TrustedRunDefinition) -> Result<([u8; 32], ProbePlans), ShadowError> {
    if definition.identity.is_empty() {
        return Err(ShadowError::InvalidInput("empty trusted run identity"));
    }
    if definition.identity.len() > definition.pool_limits.max_checkpoint_bytes
        || definition.events.len() > definition.ledger_limits.max_events
        || definition.probes.len() > definition.pool_limits.max_probes
    {
        return Err(ShadowError::LimitExceeded);
    }
    definition.events.sort_by(|a, b| a.order.cmp(&b.order));
    let payload_bytes = definition
        .events
        .iter()
        .try_fold(0usize, |n, event| n.checked_add(event.payload.len()))
        .ok_or(ShadowError::LimitExceeded)?;
    if payload_bytes > definition.ledger_limits.max_payload_bytes {
        return Err(ShadowError::LimitExceeded);
    }
    if definition.initial_resources.len() > definition.ledger_limits.max_initial_resources {
        return Err(ShadowError::LimitExceeded);
    }
    let mut identifier_bytes = 0usize;
    for assumption in &definition.assumptions {
        identifier_bytes = identifier_bytes
            .checked_add(assumption.len())
            .ok_or(ShadowError::LimitExceeded)?;
    }
    for event in &definition.events {
        for value in [
            event.order.case_key.len(),
            event.order.event_kind_rank.as_str().len(),
            event.order.source_event_key.len(),
        ] {
            identifier_bytes = identifier_bytes
                .checked_add(value)
                .ok_or(ShadowError::LimitExceeded)?;
        }
        match &event.transition {
            crate::shadow::Transition::None => {}
            crate::shadow::Transition::Acquire {
                resource, claim, ..
            }
            | crate::shadow::Transition::Release { resource, claim } => {
                identifier_bytes = identifier_bytes
                    .checked_add(resource.len())
                    .and_then(|n| n.checked_add(claim.len()))
                    .ok_or(ShadowError::LimitExceeded)?;
            }
        }
    }
    for (name, resource) in &definition.initial_resources {
        identifier_bytes = identifier_bytes
            .checked_add(name.len())
            .ok_or(ShadowError::LimitExceeded)?;
        for claim in &resource.claims {
            identifier_bytes = identifier_bytes
                .checked_add(claim.id.len())
                .ok_or(ShadowError::LimitExceeded)?;
        }
    }
    let initial_claim_count = definition
        .initial_resources
        .values()
        .try_fold(0usize, |n, resource| n.checked_add(resource.claims.len()))
        .ok_or(ShadowError::LimitExceeded)?;
    let assumption_bytes = definition
        .assumptions
        .iter()
        .try_fold(0usize, |n, value| n.checked_add(value.len()))
        .ok_or(ShadowError::LimitExceeded)?;
    if identifier_bytes > definition.ledger_limits.max_identifier_bytes
        || definition.assumptions.len() > definition.ledger_limits.max_assumptions
        || assumption_bytes > definition.ledger_limits.max_assumption_bytes
        || initial_claim_count > definition.ledger_limits.max_initial_claims
    {
        return Err(ShadowError::LimitExceeded);
    }
    let mut source = BTreeMap::new();
    for event in &definition.events {
        if event.order.source_event_key.trim().is_empty()
            || source
                .insert(event.order.source_event_key.as_str(), event)
                .is_some()
        {
            return Err(ShadowError::DuplicateIdentity(
                event.order.source_event_key.clone(),
            ));
        }
    }
    let mut estimated = definition.identity.len();
    for spec in &definition.probes {
        let key = &spec.key;
        for value in [
            spec.id.capacity(),
            spec.run_id.capacity(),
            spec.candidate_id.capacity(),
            spec.anchor_event.capacity(),
            spec.target_event.as_ref().map_or(0, String::capacity),
            spec.input.target.capacity(),
            spec.input.fidelity.capacity(),
            key.study_id.capacity(),
            key.dataset_id.capacity(),
            key.scenario_id.capacity(),
            key.seed_schedule_id.capacity(),
            key.replication_id.capacity(),
            key.case_key.capacity(),
            key.task_key.capacity(),
            key.endpoint.capacity(),
            key.seed_purpose.capacity(),
            key.seed_map_ref.capacity(),
            key.mapping_version.capacity(),
        ] {
            estimated = estimated
                .checked_add(value)
                .ok_or(ShadowError::LimitExceeded)?;
        }
        estimated = estimated
            .checked_add(std::mem::size_of::<ProbeSpec>())
            .ok_or(ShadowError::LimitExceeded)?;
    }
    if estimated > definition.pool_limits.max_checkpoint_bytes {
        return Err(ShadowError::LimitExceeded);
    }
    let mut ids = BTreeSet::new();
    let mut plans = BTreeMap::<String, Vec<ProbeSpec>>::new();
    definition.probes.sort_by(|a, b| a.id.cmp(&b.id));
    for spec in &mut definition.probes {
        if !ids.insert(spec.id.clone()) {
            return Err(ShadowError::DuplicateIdentity(spec.id.clone()));
        }
        let anchor = source
            .get(spec.anchor_event.as_str())
            .ok_or(ShadowError::InvalidInput(
                "probe anchor is not source-defined",
            ))?;
        if !anchor.source_defined {
            return Err(ShadowError::InvalidInput(
                "probe anchor is not source-defined",
            ));
        }
        let observed = match &spec.target_event {
            Some(key) => source
                .get(key.as_str())
                .filter(|event| event.source_defined)
                .map(|event| event.order.relative_ticks),
            None => None,
        };
        if let (Some(target_key), Some(target)) = (&spec.target_event, observed) {
            let target_event = source
                .get(target_key.as_str())
                .expect("target just resolved");
            if target_event.order < anchor.order {
                return Err(ShadowError::InvalidInput("target precedes probe anchor"));
            }
            spec.observed_target = Some(target);
        } else {
            spec.observed_target = None;
        }
        plans
            .entry(spec.anchor_event.clone())
            .or_default()
            .push(spec.clone());
    }
    let binding = binding_digest(definition)?;
    Ok((binding, plans))
}

fn binding_digest(definition: &TrustedRunDefinition) -> Result<[u8; 32], ShadowError> {
    let mut hash = Sha256::new();
    // Seed identity is checked by exact trusted ProbeSpec equality on restore;
    // wire encoding adds its accepted seed fingerprint in the coordinator join.
    hash.update(b"kairos.c3.trusted-run.v1\0");
    put(&mut hash, &definition.identity)?;
    hash.update((definition.events.len() as u64).to_le_bytes());
    for event in &definition.events {
        hash.update(event.order.relative_ticks.to_le_bytes());
        put(&mut hash, event.order.case_key.as_bytes())?;
        hash.update(event.order.occurrence.to_le_bytes());
        put(&mut hash, event.order.event_kind_rank.as_str().as_bytes())?;
        put(&mut hash, event.order.source_event_key.as_bytes())?;
        hash.update(event.order.source_order.to_le_bytes());
        match event.available_at {
            Some(t) => {
                hash.update([1]);
                hash.update(t.to_le_bytes());
            }
            None => hash.update([0]),
        }
        hash.update([u8::from(event.source_defined)]);
        match &event.transition {
            crate::shadow::Transition::None => hash.update([0]),
            crate::shadow::Transition::Acquire {
                resource,
                claim,
                units,
            } => {
                hash.update([1]);
                put(&mut hash, resource.as_bytes())?;
                put(&mut hash, claim.as_bytes())?;
                hash.update(units.to_le_bytes());
            }
            crate::shadow::Transition::Release { resource, claim } => {
                hash.update([2]);
                put(&mut hash, resource.as_bytes())?;
                put(&mut hash, claim.as_bytes())?;
            }
        }
        put(&mut hash, &event.payload)?;
    }
    hash.update((definition.initial_resources.len() as u64).to_le_bytes());
    for (name, resource) in &definition.initial_resources {
        put(&mut hash, name.as_bytes())?;
        hash.update(resource.capacity.to_le_bytes());
        hash.update((resource.claims.len() as u64).to_le_bytes());
        let mut claims: Vec<_> = resource.claims.iter().collect();
        claims.sort_by(|a, b| a.id.cmp(&b.id));
        for claim in claims {
            put(&mut hash, claim.id.as_bytes())?;
            hash.update(claim.units.to_le_bytes());
        }
    }
    hash.update((definition.assumptions.len() as u64).to_le_bytes());
    hash.update((definition.assumptions.len() as u64).to_le_bytes());
    for assumption in &definition.assumptions {
        put(&mut hash, assumption.as_bytes())?;
    }
    hash.update([match definition.resource_policy {
        ResourcePolicy::Strict => 0,
        ResourcePolicy::Diagnostic => 1,
    }]);
    hash.update((definition.ledger_limits.max_events as u64).to_le_bytes());
    hash.update((definition.ledger_limits.max_payload_bytes as u64).to_le_bytes());
    hash.update((definition.ledger_limits.max_assumptions as u64).to_le_bytes());
    hash.update((definition.ledger_limits.max_identifier_bytes as u64).to_le_bytes());
    hash.update((definition.ledger_limits.max_initial_resources as u64).to_le_bytes());
    hash.update((definition.ledger_limits.max_initial_claims as u64).to_le_bytes());
    hash.update((definition.ledger_limits.max_assumption_bytes as u64).to_le_bytes());
    hash.update((definition.ledger_limits.max_initial_resources as u64).to_le_bytes());
    hash.update((definition.ledger_limits.max_initial_claims as u64).to_le_bytes());
    hash.update((definition.ledger_limits.max_assumption_bytes as u64).to_le_bytes());
    hash.update((definition.pool_limits.max_probes as u64).to_le_bytes());
    hash.update((definition.pool_limits.max_snapshot_bytes as u64).to_le_bytes());
    hash.update((definition.pool_limits.max_probe_image_bytes as u64).to_le_bytes());
    hash.update((definition.pool_limits.max_checkpoint_bytes as u64).to_le_bytes());
    hash.update((definition.probes.len() as u64).to_le_bytes());
    hash.update((definition.probes.len() as u64).to_le_bytes());
    for spec in &definition.probes {
        for field in [
            spec.id.as_str(),
            spec.run_id.as_str(),
            spec.candidate_id.as_str(),
            spec.anchor_event.as_str(),
            spec.target_event.as_deref().unwrap_or(""),
            spec.input.target.as_str(),
            spec.input.fidelity.as_str(),
        ] {
            put(&mut hash, field.as_bytes())?;
        }
        hash.update(spec.input.parameter_hash);
        hash.update(spec.input.adapter_hash);
        hash.update(spec.budget.horizon.to_le_bytes());
        hash.update(spec.budget.max_events.to_le_bytes());
        let k = &spec.key;
        for field in [
            &k.study_id,
            &k.dataset_id,
            &k.scenario_id,
            &k.seed_schedule_id,
            &k.replication_id,
            &k.case_key,
            &k.task_key,
            &k.endpoint,
            &k.seed_purpose,
            &k.seed_map_ref,
            &k.mapping_version,
        ] {
            put(&mut hash, field.as_bytes())?;
        }
        hash.update(k.occurrence.to_le_bytes());
    }
    Ok(hash.finalize().into())
}

fn put(hash: &mut Sha256, bytes: &[u8]) -> Result<(), ShadowError> {
    let len = u64::try_from(bytes.len()).map_err(|_| ShadowError::LimitExceeded)?;
    hash.update(len.to_le_bytes());
    hash.update(bytes);
    Ok(())
}

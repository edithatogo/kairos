//! Bounded immutable-prefix ledger for C3 shadow replay.
use crate::shadow::{
    LedgerDiagnostic, LedgerSnapshot, ObservedEvent, ResourcePolicy, ResourceState, ShadowError,
    Transition,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InitialClaim {
    pub id: String,
    pub units: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InitialResource {
    pub capacity: u32,
    pub claims: Vec<InitialClaim>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LedgerLimits {
    pub max_events: usize,
    pub max_payload_bytes: usize,
    pub max_assumptions: usize,
    pub max_identifier_bytes: usize,
    pub max_initial_resources: usize,
    pub max_initial_claims: usize,
    pub max_assumption_bytes: usize,
}

/// Canonical source events and declared starting occupancy. Each snapshot is a
/// reconstruction from the admitted prefix, so later knowledge cannot leak back.
pub(crate) struct ObservedLedger {
    events: Vec<ObservedEvent>,
    initial: BTreeMap<String, ResourceState>,
    assumptions: Vec<String>,
    policy: ResourcePolicy,
    limits: LedgerLimits,
    frontier: usize,
    diagnostics: Vec<LedgerDiagnostic>,
    current: Option<LedgerSnapshot>,
    current_anchor_available: bool,
}

impl ObservedLedger {
    pub(crate) fn new(
        mut events: Vec<ObservedEvent>,
        initial_resources: BTreeMap<String, InitialResource>,
        assumptions: Vec<String>,
        policy: ResourcePolicy,
        limits: LedgerLimits,
    ) -> Result<Self, ShadowError> {
        if events.len() > limits.max_events
            || assumptions.len() > limits.max_assumptions
            || initial_resources.len() > limits.max_initial_resources
        {
            return Err(ShadowError::LimitExceeded);
        }
        let bytes = events
            .iter()
            .try_fold(0usize, |n, e| n.checked_add(e.payload.len()))
            .ok_or(ShadowError::LimitExceeded)?;
        if bytes > limits.max_payload_bytes {
            return Err(ShadowError::LimitExceeded);
        }
        let mut identifier_bytes = 0usize;
        for assumption in &assumptions {
            identifier_bytes = identifier_bytes
                .checked_add(assumption.len())
                .ok_or(ShadowError::LimitExceeded)?;
        }
        if identifier_bytes > limits.max_assumption_bytes {
            return Err(ShadowError::LimitExceeded);
        }
        events.sort_by(|a, b| a.order.cmp(&b.order));
        let mut keys = BTreeSet::new();
        let mut initial_claim_count = 0usize;
        for event in &events {
            for identifier in [
                event.order.case_key.as_str(),
                event.order.event_kind_rank.as_str(),
                event.order.source_event_key.as_str(),
            ] {
                identifier_bytes = identifier_bytes
                    .checked_add(identifier.len())
                    .ok_or(ShadowError::LimitExceeded)?;
            }
            match &event.transition {
                Transition::None => {}
                Transition::Acquire {
                    resource, claim, ..
                }
                | Transition::Release { resource, claim } => {
                    identifier_bytes = identifier_bytes
                        .checked_add(resource.len())
                        .and_then(|n| n.checked_add(claim.len()))
                        .ok_or(ShadowError::LimitExceeded)?;
                }
            }
            if identifier_bytes > limits.max_identifier_bytes {
                return Err(ShadowError::LimitExceeded);
            }
            if !keys.insert(event.order.source_event_key.clone()) {
                return Err(ShadowError::DuplicateIdentity(
                    event.order.source_event_key.clone(),
                ));
            }
            if let Transition::Acquire {
                resource,
                claim,
                units,
            } = &event.transition
            {
                if resource.is_empty() || claim.is_empty() || *units == 0 {
                    return Err(ShadowError::InvalidInput("invalid resource acquisition"));
                }
            }
            if let Transition::Release { resource, claim } = &event.transition {
                if resource.is_empty() || claim.is_empty() {
                    return Err(ShadowError::InvalidInput("invalid resource release"));
                }
            }
        }
        let mut initial = BTreeMap::new();
        for (resource, spec) in initial_resources {
            identifier_bytes = identifier_bytes
                .checked_add(resource.len())
                .ok_or(ShadowError::LimitExceeded)?;
            if identifier_bytes > limits.max_identifier_bytes {
                return Err(ShadowError::LimitExceeded);
            }
            initial_claim_count = initial_claim_count
                .checked_add(spec.claims.len())
                .ok_or(ShadowError::LimitExceeded)?;
            if initial_claim_count > limits.max_initial_claims {
                return Err(ShadowError::LimitExceeded);
            }
            if resource.is_empty() || spec.capacity == 0 {
                return Err(ShadowError::InvalidInput(
                    "resource capacity must be positive",
                ));
            }
            let mut claims = BTreeMap::new();
            for claim in spec.claims {
                identifier_bytes = identifier_bytes
                    .checked_add(claim.id.len())
                    .ok_or(ShadowError::LimitExceeded)?;
                if identifier_bytes > limits.max_identifier_bytes {
                    return Err(ShadowError::LimitExceeded);
                }
                if claim.id.is_empty()
                    || claim.units == 0
                    || claims.insert(claim.id, claim.units).is_some()
                {
                    return Err(ShadowError::InvalidInput(
                        "invalid or duplicate initial claim",
                    ));
                }
            }
            let occupied = claims
                .values()
                .try_fold(0u32, |sum, units| sum.checked_add(*units))
                .ok_or(ShadowError::LimitExceeded)?;
            if occupied > spec.capacity {
                return Err(ShadowError::InvalidInput(
                    "initial occupancy exceeds capacity",
                ));
            }
            initial.insert(
                resource,
                ResourceState {
                    capacity: spec.capacity,
                    claims,
                },
            );
        }
        Ok(Self {
            events,
            initial,
            assumptions,
            policy,
            limits,
            frontier: 0,
            diagnostics: Vec::new(),
            current: None,
            current_anchor_available: false,
        })
    }

    /// Advance through exactly one canonical source row and materialize the
    /// post-row view at its occurrence tick. Strict rejection is transactional.
    pub(crate) fn advance(&mut self) -> Result<Option<&LedgerSnapshot>, ShadowError> {
        if self.frontier >= self.events.len() {
            return Ok(None);
        }
        let index = self.frontier;
        let event = self.events[index].clone();
        let anchor_key = event.order.source_event_key.clone();
        let tick = event.order.relative_ticks;
        let visible: Vec<_> = self.events[..=index]
            .iter()
            .filter(|e| {
                e.source_defined
                    && e.order.relative_ticks <= tick
                    && e.available_at.is_some_and(|known| known <= tick)
            })
            .cloned()
            .collect();
        let historical: Vec<_> = self.events[..=index]
            .iter()
            .filter(|e| e.source_defined)
            .cloned()
            .collect();
        let (_, historical_diagnostics) = self.rebuild_resources(&historical);
        if self.policy == ResourcePolicy::Strict && !historical_diagnostics.is_empty() {
            let d = historical_diagnostics[0].clone();
            self.record_diagnostics(historical_diagnostics);
            return Err(ShadowError::ResourceInfeasible(d));
        }
        self.record_diagnostics(historical_diagnostics);
        let (resources, diagnostics) = self.rebuild_resources(&visible);
        let mut snapshot_assumptions = self.assumptions.clone();
        for excluded in self.events[..=index].iter().filter(|e| {
            e.source_defined
                && !matches!(e.transition, Transition::None)
                && !visible
                    .iter()
                    .any(|v| v.order.source_event_key == e.order.source_event_key)
        }) {
            snapshot_assumptions.push(format!(
                "resource occupancy excluded for unavailable event {}",
                excluded.order.source_event_key
            ));
        }
        if snapshot_assumptions.len() > self.limits.max_assumptions
            || snapshot_assumptions
                .iter()
                .try_fold(0usize, |n, s| n.checked_add(s.len()))
                .ok_or(ShadowError::LimitExceeded)?
                > self.limits.max_assumption_bytes
        {
            return Err(ShadowError::LimitExceeded);
        }
        let anchor_available =
            event.source_defined && event.available_at.is_some_and(|known| known <= tick);
        let mut snapshot = LedgerSnapshot {
            frontier: index + 1,
            at: tick,
            anchor_event: anchor_key.clone(),
            digest: [0; 32],
            visible_events: visible,
            resources,
            resource_feasible: diagnostics.is_empty(),
            assumptions: snapshot_assumptions,
        };
        snapshot.digest = digest(&snapshot);
        self.frontier = index + 1;
        self.record_diagnostics(diagnostics);
        self.current = Some(snapshot);
        self.current_anchor_available = anchor_available;
        if !anchor_available {
            return Err(ShadowError::UnavailableAnchor(anchor_key));
        }
        Ok(self.current.as_ref())
    }

    pub(crate) fn current(&self) -> Result<Option<&LedgerSnapshot>, ShadowError> {
        match self.current.as_ref() {
            Some(snapshot) if !self.current_anchor_available => Err(
                ShadowError::UnavailableAnchor(snapshot.anchor_event.clone()),
            ),
            current => Ok(current),
        }
    }
    pub(crate) fn frontier(&self) -> usize {
        self.frontier
    }
    pub(crate) fn diagnostics(&self) -> &[LedgerDiagnostic] {
        &self.diagnostics
    }

    fn record_diagnostics(&mut self, incoming: Vec<LedgerDiagnostic>) {
        for diagnostic in incoming {
            if !self.diagnostics.contains(&diagnostic) {
                self.diagnostics.push(diagnostic);
            }
        }
    }

    fn rebuild_resources(
        &self,
        visible: &[ObservedEvent],
    ) -> (BTreeMap<String, ResourceState>, Vec<LedgerDiagnostic>) {
        let mut resources = self.initial.clone();
        let mut diagnostics = Vec::new();
        for event in visible {
            match &event.transition {
                Transition::None => {}
                Transition::Acquire {
                    resource,
                    claim,
                    units,
                } => {
                    let Some(state) = resources.get_mut(resource) else {
                        diagnostics.push(diag(event, "unknown resource"));
                        continue;
                    };
                    if state.claims.contains_key(claim) {
                        diagnostics.push(diag(event, "duplicate active claim"));
                        continue;
                    }
                    let used = state
                        .claims
                        .values()
                        .try_fold(0u32, |sum, n| sum.checked_add(*n));
                    let Some(next) = used.and_then(|n| n.checked_add(*units)) else {
                        diagnostics.push(diag(event, "occupancy overflow"));
                        continue;
                    };
                    if next > state.capacity {
                        diagnostics.push(diag(event, "occupancy exceeds capacity"));
                        continue;
                    }
                    state.claims.insert(claim.clone(), *units);
                }
                Transition::Release { resource, claim } => {
                    let Some(state) = resources.get_mut(resource) else {
                        diagnostics.push(diag(event, "unknown resource"));
                        continue;
                    };
                    if state.claims.remove(claim).is_none() {
                        diagnostics.push(diag(event, "release names no active claim"));
                    }
                }
            }
        }
        (resources, diagnostics)
    }
}

fn diag(event: &ObservedEvent, reason: &str) -> LedgerDiagnostic {
    LedgerDiagnostic {
        event: event.order.source_event_key.clone(),
        reason: reason.to_owned(),
    }
}

fn digest(s: &LedgerSnapshot) -> [u8; 32] {
    fn put(h: &mut Sha256, b: &[u8]) {
        h.update((b.len() as u64).to_le_bytes());
        h.update(b);
    }
    fn num(h: &mut Sha256, n: u128) {
        h.update(n.to_le_bytes());
    }
    let mut h = Sha256::new();
    h.update(b"kairos.c3.ledger-snapshot.v1\0");
    h.update((s.frontier as u64).to_le_bytes());
    num(&mut h, s.at);
    put(&mut h, s.anchor_event.as_bytes());
    h.update([u8::from(s.resource_feasible)]);
    h.update((s.visible_events.len() as u64).to_le_bytes());
    for e in &s.visible_events {
        num(&mut h, e.order.relative_ticks);
        put(&mut h, e.order.case_key.as_bytes());
        h.update(e.order.occurrence.to_le_bytes());
        put(&mut h, e.order.event_kind_rank.as_str().as_bytes());
        put(&mut h, e.order.source_event_key.as_bytes());
        h.update(e.order.source_order.to_le_bytes());
        match e.available_at {
            Some(t) => {
                h.update([1]);
                num(&mut h, t)
            }
            None => h.update([0]),
        }
        h.update([u8::from(e.source_defined)]);
        match &e.transition {
            Transition::None => h.update([0]),
            Transition::Acquire {
                resource,
                claim,
                units,
            } => {
                h.update([1]);
                put(&mut h, resource.as_bytes());
                put(&mut h, claim.as_bytes());
                h.update(units.to_le_bytes())
            }
            Transition::Release { resource, claim } => {
                h.update([2]);
                put(&mut h, resource.as_bytes());
                put(&mut h, claim.as_bytes())
            }
        }
        put(&mut h, &e.payload);
    }
    h.update((s.resources.len() as u64).to_le_bytes());
    for (name, r) in &s.resources {
        put(&mut h, name.as_bytes());
        h.update(r.capacity.to_le_bytes());
        h.update((r.claims.len() as u64).to_le_bytes());
        for (id, units) in &r.claims {
            put(&mut h, id.as_bytes());
            h.update(units.to_le_bytes());
        }
    }
    h.update((s.assumptions.len() as u64).to_le_bytes());
    for a in &s.assumptions {
        put(&mut h, a.as_bytes());
    }
    h.finalize().into()
}

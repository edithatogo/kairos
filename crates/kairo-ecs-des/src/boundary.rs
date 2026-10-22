#![forbid(unsafe_code)]

use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, SimTime};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Unique operation identifier for an inter-unit entity handover.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HandoverOperationId(pub u64);

impl HandoverOperationId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for HandoverOperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HandoverOp({})", self.0)
    }
}

/// Lifecycle states of an inter-unit entity transfer.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TransferState {
    Requested,
    Offered,
    Reserved,
    Departing,
    InTransit,
    Arrived,
    Acknowledged,
    Rejected,
    Cancelled,
    Expired,
}

impl TransferState {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Acknowledged | Self::Rejected | Self::Cancelled | Self::Expired
        )
    }

    pub const fn is_active_in_transit(self) -> bool {
        matches!(self, Self::Departing | Self::InTransit | Self::Arrived)
    }
}

/// Parametric distribution for stochastic inter-unit transit times.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransitDelayDistribution {
    Fixed(SimTime),
    Uniform { min: SimTime, max: SimTime },
    Exponential { mean_ticks: u128 },
}

impl TransitDelayDistribution {
    pub fn sample(&self, rng: &mut DeterministicStream) -> SimTime {
        match *self {
            Self::Fixed(delay) => delay,
            Self::Uniform { min, max } => {
                let min_ticks = min.ticks();
                let max_ticks = max.ticks();
                if min_ticks >= max_ticks {
                    return min;
                }
                let span = max_ticks - min_ticks;
                let draw = (rng.next_u64() as u128) % (span + 1);
                SimTime::from_ticks(min_ticks + draw)
            }
            Self::Exponential { mean_ticks } => {
                if mean_ticks == 0 {
                    return SimTime::ZERO;
                }
                // Deterministic inverse transform sampling: t = -mean * ln(1 - U)
                let u: f64 = ((rng.next_u64() >> 11) as f64 + 1.0) / ((1u64 << 53) as f64 + 2.0);
                let delay_float: f64 = -(mean_ticks as f64) * (1.0 - u).ln();
                let ticks = delay_float.round().max(0.0) as u128;
                SimTime::from_ticks(ticks)
            }
        }
    }
}

/// Neutral payload envelope carrying entity transfer metadata across boundary channels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandoverEnvelope<T> {
    pub operation_id: HandoverOperationId,
    pub entity: EntityId,
    pub payload: T,
    pub sender_unit: String,
    pub receiver_unit: String,
    pub state: TransferState,
    pub requested_at: SimTime,
    pub depart_at: Option<SimTime>,
    pub arrive_at: Option<SimTime>,
    pub acknowledged_at: Option<SimTime>,
    pub timeout: Option<SimTime>,
    pub retry_count: u32,
}

impl<T> HandoverEnvelope<T> {
    pub fn new(
        operation_id: HandoverOperationId,
        entity: EntityId,
        payload: T,
        sender_unit: impl Into<String>,
        receiver_unit: impl Into<String>,
        requested_at: SimTime,
        timeout: Option<SimTime>,
    ) -> Self {
        Self {
            operation_id,
            entity,
            payload,
            sender_unit: sender_unit.into(),
            receiver_unit: receiver_unit.into(),
            state: TransferState::Requested,
            requested_at,
            depart_at: None,
            arrive_at: None,
            acknowledged_at: None,
            timeout,
            retry_count: 0,
        }
    }
}

/// Errors occurring during boundary channel transfer operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundaryError {
    CapacityExceeded {
        max_capacity: usize,
        current: usize,
    },
    OperationNotFound(HandoverOperationId),
    InvalidStateTransition {
        op_id: HandoverOperationId,
        current: TransferState,
        attempted: TransferState,
    },
    OperationAlreadyExists(HandoverOperationId),
    TransferExpired(HandoverOperationId),
    TransferCancelled(HandoverOperationId),
    DestinationUnavailable(String),
}

impl fmt::Display for BoundaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded {
                max_capacity,
                current,
            } => write!(
                f,
                "boundary channel capacity exceeded: max {}, current {}",
                max_capacity, current
            ),
            Self::OperationNotFound(op) => write!(f, "handover operation not found: {}", op),
            Self::InvalidStateTransition {
                op_id,
                current,
                attempted,
            } => write!(
                f,
                "invalid handover state transition for {}: {:?} -> {:?}",
                op_id, current, attempted
            ),
            Self::OperationAlreadyExists(op) => {
                write!(f, "handover operation already exists: {}", op)
            }
            Self::TransferExpired(op) => write!(f, "handover transfer expired: {}", op),
            Self::TransferCancelled(op) => write!(f, "handover transfer cancelled: {}", op),
            Self::DestinationUnavailable(dest) => {
                write!(f, "handover destination unavailable: {}", dest)
            }
        }
    }
}

impl std::error::Error for BoundaryError {}

/// Asynchronous cross-boundary transit channel between decoupled simulation units.
pub struct BoundaryChannel<T> {
    name: String,
    max_in_transit: usize,
    delay_distribution: TransitDelayDistribution,
    in_transit: Vec<HandoverEnvelope<T>>,
    ledger: HashMap<HandoverOperationId, HandoverEnvelope<T>>,
    completed_ops: HashSet<HandoverOperationId>,
}

impl<T: Clone> BoundaryChannel<T> {
    pub fn new(
        name: impl Into<String>,
        max_in_transit: usize,
        delay_distribution: TransitDelayDistribution,
    ) -> Self {
        Self {
            name: name.into(),
            max_in_transit,
            delay_distribution,
            in_transit: Vec::with_capacity(max_in_transit),
            ledger: HashMap::new(),
            completed_ops: HashSet::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn max_in_transit(&self) -> usize {
        self.max_in_transit
    }

    pub fn in_transit_count(&self) -> usize {
        self.in_transit.len()
    }

    pub fn available_channel_capacity(&self) -> usize {
        self.max_in_transit.saturating_sub(self.in_transit.len())
    }

    pub fn get_envelope(&self, op_id: HandoverOperationId) -> Option<&HandoverEnvelope<T>> {
        self.ledger.get(&op_id)
    }

    /// Submits a transfer request into the channel.
    /// Idempotency guarantee: If the operation was already requested or completed,
    /// returns the existing envelope without creating duplicate transfers.
    pub fn request_transfer(
        &mut self,
        envelope: HandoverEnvelope<T>,
    ) -> Result<TransferState, BoundaryError> {
        let op_id = envelope.operation_id;
        if self.completed_ops.contains(&op_id) {
            if let Some(existing) = self.ledger.get(&op_id) {
                return Ok(existing.state);
            }
        }
        if let Some(existing) = self.ledger.get(&op_id) {
            return Ok(existing.state);
        }

        self.ledger.insert(op_id, envelope);
        Ok(TransferState::Requested)
    }

    /// Reserves destination capacity and transitions state to `Reserved`.
    pub fn reserve(&mut self, op_id: HandoverOperationId) -> Result<(), BoundaryError> {
        let env = self
            .ledger
            .get_mut(&op_id)
            .ok_or(BoundaryError::OperationNotFound(op_id))?;

        if env.state != TransferState::Requested && env.state != TransferState::Offered {
            return Err(BoundaryError::InvalidStateTransition {
                op_id,
                current: env.state,
                attempted: TransferState::Reserved,
            });
        }

        env.state = TransferState::Reserved;
        Ok(())
    }

    /// Departs an entity into transit across the channel, sampling transit delay.
    pub fn depart(
        &mut self,
        op_id: HandoverOperationId,
        now: SimTime,
        rng: &mut DeterministicStream,
    ) -> Result<SimTime, BoundaryError> {
        if self.in_transit.len() >= self.max_in_transit {
            return Err(BoundaryError::CapacityExceeded {
                max_capacity: self.max_in_transit,
                current: self.in_transit.len(),
            });
        }

        let env = self
            .ledger
            .get_mut(&op_id)
            .ok_or(BoundaryError::OperationNotFound(op_id))?;

        if let Some(timeout) = env.timeout {
            if now > timeout {
                env.state = TransferState::Expired;
                self.completed_ops.insert(op_id);
                return Err(BoundaryError::TransferExpired(op_id));
            }
        }

        if env.state != TransferState::Reserved
            && env.state != TransferState::Requested
            && env.state != TransferState::Offered
        {
            return Err(BoundaryError::InvalidStateTransition {
                op_id,
                current: env.state,
                attempted: TransferState::InTransit,
            });
        }

        let delay = self.delay_distribution.sample(rng);
        let arrival_time = SimTime::from_ticks(now.ticks() + delay.ticks());

        env.state = TransferState::InTransit;
        env.depart_at = Some(now);
        env.arrive_at = Some(arrival_time);

        let active_env = env.clone();
        self.in_transit.push(active_env);

        Ok(arrival_time)
    }

    /// Collects all envelopes that have reached arrival time by `now`.
    pub fn collect_arrivals(&mut self, now: SimTime) -> Vec<HandoverEnvelope<T>> {
        let mut arrived = Vec::new();
        let mut remaining = Vec::with_capacity(self.in_transit.len());

        for mut env in self.in_transit.drain(..) {
            if let Some(arr) = env.arrive_at {
                if arr <= now {
                    env.state = TransferState::Arrived;
                    if let Some(ledger_env) = self.ledger.get_mut(&env.operation_id) {
                        ledger_env.state = TransferState::Arrived;
                    }
                    arrived.push(env);
                    continue;
                }
            }
            remaining.push(env);
        }

        self.in_transit = remaining;
        arrived
    }

    /// Acknowledges transfer completion by receiver.
    /// Finalizes custody handover and releases channel resources.
    pub fn acknowledge(
        &mut self,
        op_id: HandoverOperationId,
        now: SimTime,
    ) -> Result<HandoverEnvelope<T>, BoundaryError> {
        let env = self
            .ledger
            .get_mut(&op_id)
            .ok_or(BoundaryError::OperationNotFound(op_id))?;

        if env.state != TransferState::Arrived && env.state != TransferState::InTransit {
            return Err(BoundaryError::InvalidStateTransition {
                op_id,
                current: env.state,
                attempted: TransferState::Acknowledged,
            });
        }

        env.state = TransferState::Acknowledged;
        env.acknowledged_at = Some(now);
        self.completed_ops.insert(op_id);

        self.in_transit.retain(|e| e.operation_id != op_id);
        Ok(env.clone())
    }

    /// Rejects transfer by receiver or intermediate filter, triggering compensation.
    pub fn reject(
        &mut self,
        op_id: HandoverOperationId,
        _reason: &str,
    ) -> Result<HandoverEnvelope<T>, BoundaryError> {
        let env = self
            .ledger
            .get_mut(&op_id)
            .ok_or(BoundaryError::OperationNotFound(op_id))?;

        if env.state.is_terminal() {
            return Err(BoundaryError::InvalidStateTransition {
                op_id,
                current: env.state,
                attempted: TransferState::Rejected,
            });
        }

        env.state = TransferState::Rejected;
        self.completed_ops.insert(op_id);
        self.in_transit.retain(|e| e.operation_id != op_id);
        Ok(env.clone())
    }

    /// Cancels transfer before completion, triggering sender resource retention.
    pub fn cancel(
        &mut self,
        op_id: HandoverOperationId,
    ) -> Result<HandoverEnvelope<T>, BoundaryError> {
        let env = self
            .ledger
            .get_mut(&op_id)
            .ok_or(BoundaryError::OperationNotFound(op_id))?;

        if env.state.is_terminal() {
            return Err(BoundaryError::InvalidStateTransition {
                op_id,
                current: env.state,
                attempted: TransferState::Cancelled,
            });
        }

        env.state = TransferState::Cancelled;
        self.completed_ops.insert(op_id);
        self.in_transit.retain(|e| e.operation_id != op_id);
        Ok(env.clone())
    }

    /// Checks for and transitions expired transfers whose timeout has passed.
    pub fn check_expiries(&mut self, now: SimTime) -> Vec<HandoverOperationId> {
        let mut expired = Vec::new();
        for (op_id, env) in &mut self.ledger {
            if !env.state.is_terminal() {
                if let Some(timeout) = env.timeout {
                    if now > timeout {
                        env.state = TransferState::Expired;
                        expired.push(*op_id);
                    }
                }
            }
        }

        for &op_id in &expired {
            self.completed_ops.insert(op_id);
            self.in_transit.retain(|e| e.operation_id != op_id);
        }

        expired
    }
}

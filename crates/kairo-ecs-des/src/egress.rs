#![forbid(unsafe_code)]

use crate::boundary::{BoundaryError, HandoverEnvelope, HandoverOperationId, TransferState};
use kairo_ecs_types::{EntityId, SimTime};
use std::collections::{HashMap, VecDeque};

/// Telemetry event capturing inter-unit handover state transitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandoverTelemetryEvent {
    ReservationRequested {
        op_id: HandoverOperationId,
        entity: EntityId,
        at: SimTime,
    },
    ReservationGranted {
        op_id: HandoverOperationId,
        entity: EntityId,
        at: SimTime,
    },
    BoardingHoldEntered {
        op_id: HandoverOperationId,
        entity: EntityId,
        at: SimTime,
    },
    BoardingHoldReleased {
        op_id: HandoverOperationId,
        entity: EntityId,
        at: SimTime,
    },
    PhysicalArrival {
        op_id: HandoverOperationId,
        entity: EntityId,
        at: SimTime,
    },
    HandoverAcknowledged {
        op_id: HandoverOperationId,
        entity: EntityId,
        at: SimTime,
    },
    HandoverCompensated {
        op_id: HandoverOperationId,
        entity: EntityId,
        reason: String,
        at: SimTime,
    },
}

/// Asynchronous egress sink and handover intake managing receiver capacity,
/// boarding holds, and explicit reservation vs physical occupancy accounting.
pub struct EgressSink<T> {
    name: String,
    capacity: usize,
    reserved_count: usize,
    occupied_count: usize,
    boarding_queue: VecDeque<HandoverEnvelope<T>>,
    reservations: HashMap<HandoverOperationId, HandoverEnvelope<T>>,
    physically_present: HashMap<EntityId, HandoverEnvelope<T>>,
    telemetry: Vec<HandoverTelemetryEvent>,
}

impl<T: Clone> EgressSink<T> {
    pub fn new(name: impl Into<String>, capacity: usize) -> Self {
        Self {
            name: name.into(),
            capacity,
            reserved_count: 0,
            occupied_count: 0,
            boarding_queue: VecDeque::new(),
            reservations: HashMap::new(),
            physically_present: HashMap::new(),
            telemetry: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn reserved_count(&self) -> usize {
        self.reserved_count
    }

    pub fn occupied_count(&self) -> usize {
        self.occupied_count
    }

    pub fn total_committed(&self) -> usize {
        self.reserved_count + self.occupied_count
    }

    pub fn available_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.total_committed())
    }

    pub fn boarding_queue_len(&self) -> usize {
        self.boarding_queue.len()
    }

    pub fn has_boarding_hold(&self, op_id: HandoverOperationId) -> bool {
        self.boarding_queue.iter().any(|e| e.operation_id == op_id)
    }

    pub fn is_physically_present(&self, entity: EntityId) -> bool {
        self.physically_present.contains_key(&entity)
    }

    /// Attempts to reserve destination capacity for an incoming transfer.
    /// If capacity is available, grants immediate reservation.
    /// If capacity is full, places the handover in boarding hold at sender.
    pub fn request_intake(
        &mut self,
        mut envelope: HandoverEnvelope<T>,
        now: SimTime,
    ) -> Result<TransferState, BoundaryError> {
        let op_id = envelope.operation_id;
        let entity = envelope.entity;

        self.telemetry
            .push(HandoverTelemetryEvent::ReservationRequested {
                op_id,
                entity,
                at: now,
            });

        if self.available_capacity() > 0 {
            self.reserved_count += 1;
            envelope.state = TransferState::Reserved;
            self.reservations.insert(op_id, envelope);

            self.telemetry
                .push(HandoverTelemetryEvent::ReservationGranted {
                    op_id,
                    entity,
                    at: now,
                });

            Ok(TransferState::Reserved)
        } else {
            envelope.state = TransferState::Offered;
            self.boarding_queue.push_back(envelope);

            self.telemetry
                .push(HandoverTelemetryEvent::BoardingHoldEntered {
                    op_id,
                    entity,
                    at: now,
                });

            Ok(TransferState::Offered)
        }
    }

    /// Ingests physical arrival of an entity.
    /// Converts reserved capacity into physical occupancy (`reserved_count -= 1`, `occupied_count += 1`).
    pub fn receive_arrival(
        &mut self,
        mut envelope: HandoverEnvelope<T>,
        now: SimTime,
    ) -> Result<(), BoundaryError> {
        let op_id = envelope.operation_id;
        let entity = envelope.entity;

        // Verify or reconcile reservation
        if self.reservations.remove(&op_id).is_some() {
            self.reserved_count = self.reserved_count.saturating_sub(1);
        } else if self.occupied_count >= self.capacity {
            return Err(BoundaryError::CapacityExceeded {
                max_capacity: self.capacity,
                current: self.occupied_count,
            });
        }

        self.occupied_count += 1;
        envelope.state = TransferState::Arrived;
        envelope.arrive_at = Some(now);

        self.physically_present.insert(entity, envelope);

        self.telemetry
            .push(HandoverTelemetryEvent::PhysicalArrival {
                op_id,
                entity,
                at: now,
            });

        Ok(())
    }

    /// Acknowledges custody receipt at receiver unit.
    /// Officially releases sender responsibility and boarding hold.
    pub fn acknowledge_handover(
        &mut self,
        op_id: HandoverOperationId,
        now: SimTime,
    ) -> Result<HandoverEnvelope<T>, BoundaryError> {
        let env = self
            .physically_present
            .values_mut()
            .find(|e| e.operation_id == op_id)
            .ok_or(BoundaryError::OperationNotFound(op_id))?;

        env.state = TransferState::Acknowledged;
        env.acknowledged_at = Some(now);
        let confirmed = env.clone();

        self.telemetry
            .push(HandoverTelemetryEvent::HandoverAcknowledged {
                op_id,
                entity: confirmed.entity,
                at: now,
            });

        Ok(confirmed)
    }

    /// Releases destination occupancy when entity exits the receiver unit (e.g. discharge/transfer).
    /// If boarding queue has waiting transfers, automatically promotes the next one to `Reserved`.
    pub fn release_occupancy(
        &mut self,
        entity: EntityId,
        now: SimTime,
    ) -> Option<HandoverEnvelope<T>> {
        let removed = self.physically_present.remove(&entity)?;
        self.occupied_count = self.occupied_count.saturating_sub(1);

        // Check boarding queue for waiting transfers
        if let Some(mut waiting) = self.boarding_queue.pop_front() {
            self.reserved_count += 1;
            waiting.state = TransferState::Reserved;
            let op_id = waiting.operation_id;
            let waiting_entity = waiting.entity;

            self.reservations.insert(op_id, waiting.clone());

            self.telemetry
                .push(HandoverTelemetryEvent::BoardingHoldReleased {
                    op_id,
                    entity: waiting_entity,
                    at: now,
                });

            self.telemetry
                .push(HandoverTelemetryEvent::ReservationGranted {
                    op_id,
                    entity: waiting_entity,
                    at: now,
                });
        }

        Some(removed)
    }

    /// Compensates for cancelled, rejected, or expired transfers.
    /// Releases reserved capacity and promotes waiting boarding transfers if any.
    pub fn compensate_failure(&mut self, op_id: HandoverOperationId, reason: &str, now: SimTime) {
        let mut found_entity = None;

        if let Some(env) = self.reservations.remove(&op_id) {
            self.reserved_count = self.reserved_count.saturating_sub(1);
            found_entity = Some(env.entity);
        } else if let Some(pos) = self
            .boarding_queue
            .iter()
            .position(|e| e.operation_id == op_id)
        {
            let env = self.boarding_queue.remove(pos).unwrap();
            found_entity = Some(env.entity);
        }

        if let Some(entity) = found_entity {
            self.telemetry
                .push(HandoverTelemetryEvent::HandoverCompensated {
                    op_id,
                    entity,
                    reason: reason.to_string(),
                    at: now,
                });

            // Promote waiting transfer from boarding hold if capacity opened up
            if self.available_capacity() > 0 {
                if let Some(mut waiting) = self.boarding_queue.pop_front() {
                    self.reserved_count += 1;
                    waiting.state = TransferState::Reserved;
                    let wait_op = waiting.operation_id;
                    let wait_ent = waiting.entity;

                    self.reservations.insert(wait_op, waiting);

                    self.telemetry
                        .push(HandoverTelemetryEvent::BoardingHoldReleased {
                            op_id: wait_op,
                            entity: wait_ent,
                            at: now,
                        });
                    self.telemetry
                        .push(HandoverTelemetryEvent::ReservationGranted {
                            op_id: wait_op,
                            entity: wait_ent,
                            at: now,
                        });
                }
            }
        }
    }

    /// Access collected telemetry events.
    pub fn telemetry(&self) -> &[HandoverTelemetryEvent] {
        &self.telemetry
    }

    /// Drain collected telemetry events for export.
    pub fn drain_telemetry(&mut self) -> Vec<HandoverTelemetryEvent> {
        std::mem::take(&mut self.telemetry)
    }
}

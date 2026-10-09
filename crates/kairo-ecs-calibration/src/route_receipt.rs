//! Private, experimental identity for configured single-profile transit routes.
//!
//! The receipt hashes the immutable route retained by an actual transit carrier.
//! It is not a seed identity, checkpoint codec, or authenticity signature.

use kairo_ecs_abm::spatial::{RoutePlan, RouteSegment};
use kairo_ecs_abm::TransitContext;
use sha2::{Digest, Sha256};

const RECEIPT_TAG: &[u8] = b"KAIROS-CALIBRATION-ROUTE-RECEIPT\0";
const RECEIPT_VERSION: u32 = 1;
pub(crate) const ROUTE_METADATA_VERSION_V1: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DistanceProvenance {
    ConfiguredGeometry,
    SensorObservationOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RouteReceiptError {
    UnsupportedMetadataVersion,
    InvalidTripPurpose,
    SensorObservationOnly,
    RouteMismatch,
    EncodingOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RouteMetadata {
    version: u32,
    trip_purpose: String,
    distance_provenance: DistanceProvenance,
}

impl RouteMetadata {
    pub(crate) fn v1(
        trip_purpose: impl Into<String>,
        distance_provenance: DistanceProvenance,
    ) -> Self {
        Self {
            version: ROUTE_METADATA_VERSION_V1,
            trip_purpose: trip_purpose.into(),
            distance_provenance,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_version(
        version: u32,
        trip_purpose: impl Into<String>,
        distance_provenance: DistanceProvenance,
    ) -> Self {
        Self {
            version,
            trip_purpose: trip_purpose.into(),
            distance_provenance,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), RouteReceiptError> {
        if self.version != ROUTE_METADATA_VERSION_V1 {
            return Err(RouteReceiptError::UnsupportedMetadataVersion);
        }
        validate_purpose(&self.trip_purpose)?;
        if self.distance_provenance == DistanceProvenance::SensorObservationOnly {
            return Err(RouteReceiptError::SensorObservationOnly);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RouteReceipt {
    metadata: RouteMetadata,
    canonical_bytes: Vec<u8>,
    sha256: [u8; 32],
}

impl RouteReceipt {
    pub(crate) fn from_context(
        context: &TransitContext,
        metadata: &RouteMetadata,
    ) -> Result<Self, RouteReceiptError> {
        metadata.validate()?;
        let canonical_bytes = canonical_receipt_bytes(context.route_plan(), metadata)?;
        let sha256 = Sha256::digest(&canonical_bytes).into();
        Ok(Self {
            metadata: metadata.clone(),
            canonical_bytes,
            sha256,
        })
    }

    /// Validates against the actual immutable plan retained by the carrier.
    /// No caller-provided digest or copied request topology is accepted.
    pub(crate) fn validate_context(
        &self,
        context: &TransitContext,
        metadata: &RouteMetadata,
    ) -> Result<(), RouteReceiptError> {
        metadata.validate()?;
        if &self.metadata != metadata {
            return Err(RouteReceiptError::RouteMismatch);
        }
        let actual = canonical_receipt_bytes(context.route_plan(), metadata)?;
        let digest: [u8; 32] = Sha256::digest(&actual).into();
        if actual != self.canonical_bytes || digest != self.sha256 {
            return Err(RouteReceiptError::RouteMismatch);
        }
        Ok(())
    }

    #[cfg(test)]
    fn digest(&self) -> &[u8; 32] {
        &self.sha256
    }
}

fn validate_purpose(purpose: &str) -> Result<(), RouteReceiptError> {
    if purpose.is_empty()
        || purpose.len() > 1024
        || purpose.trim() != purpose
        || purpose.chars().any(char::is_control)
    {
        return Err(RouteReceiptError::InvalidTripPurpose);
    }
    Ok(())
}

fn canonical_receipt_bytes(
    route: &RoutePlan,
    metadata: &RouteMetadata,
) -> Result<Vec<u8>, RouteReceiptError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(RECEIPT_TAG);
    bytes.extend_from_slice(&RECEIPT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&metadata.version.to_le_bytes());
    append_bytes(&mut bytes, metadata.trip_purpose.as_bytes())?;
    bytes.push(match metadata.distance_provenance {
        DistanceProvenance::ConfiguredGeometry => 1,
        DistanceProvenance::SensorObservationOnly => 2,
    });
    append_bytes(&mut bytes, route.graph_canonical_bytes())?;
    bytes.extend_from_slice(&route.graph_version().to_le_bytes());
    append_bytes(&mut bytes, route.profile().mode().as_str().as_bytes())?;
    bytes.extend_from_slice(&route.profile().speed_mm_per_second().get().to_le_bytes());
    bytes.extend_from_slice(&route.ticks_per_second().to_le_bytes());
    bytes.extend_from_slice(&route.origin().value().to_le_bytes());
    bytes.extend_from_slice(&route.destination().value().to_le_bytes());
    bytes.extend_from_slice(&route.distance_mm().to_le_bytes());
    bytes.extend_from_slice(&route.duration().ticks().to_le_bytes());
    let segment_count =
        u64::try_from(route.segments().len()).map_err(|_| RouteReceiptError::EncodingOverflow)?;
    bytes.extend_from_slice(&segment_count.to_le_bytes());
    for segment in route.segments() {
        append_segment(&mut bytes, segment);
    }
    Ok(bytes)
}

fn append_bytes(target: &mut Vec<u8>, value: &[u8]) -> Result<(), RouteReceiptError> {
    let length = u64::try_from(value.len()).map_err(|_| RouteReceiptError::EncodingOverflow)?;
    target.extend_from_slice(&length.to_le_bytes());
    target.extend_from_slice(value);
    Ok(())
}

fn append_segment(target: &mut Vec<u8>, segment: &RouteSegment) {
    target.extend_from_slice(&segment.edge_id().value().to_le_bytes());
    target.extend_from_slice(&segment.from().value().to_le_bytes());
    target.extend_from_slice(&segment.to().value().to_le_bytes());
    target.extend_from_slice(&segment.length_mm().to_le_bytes());
    target.extend_from_slice(&segment.start_offset().ticks().to_le_bytes());
    target.extend_from_slice(&segment.end_offset().ticks().to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_ecs_abm::spatial::{
        EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitGraphV1,
    };
    use kairo_ecs_abm::{
        register_transit_context, schedule_transit_control, schedule_transit_start, TransitPhase,
    };
    use kairo_ecs_des::{FlowAcquireCommand, FlowBatchReceipt, FlowDomainControl, FlowRuntime};
    use kairo_ecs_types::{EventKind, SimDuration, SimTime};

    const KIND: EventKind = EventKind::custom(9411);

    fn route_metadata(purpose: &str) -> RouteMetadata {
        RouteMetadata::v1(purpose, DistanceProvenance::ConfiguredGeometry)
    }

    fn route_graph(lengths: [u64; 2], reversed: bool, speed: u64) -> RoutePlan {
        route_graph_at(lengths, reversed, speed, 1, 1, 3, "walk")
    }

    fn route_graph_at(
        lengths: [u64; 2],
        reversed: bool,
        speed: u64,
        ticks_per_second: u64,
        origin: u64,
        destination: u64,
        mode_name: &str,
    ) -> RoutePlan {
        let nodes = [NodeId::new(1), NodeId::new(2), NodeId::new(3)];
        let mut graph_nodes = nodes;
        if reversed {
            graph_nodes.reverse();
        }
        let walk = MovementModeId::new("walk").unwrap();
        let roll = MovementModeId::new("roll").unwrap();
        let allowed_modes = vec![roll, walk];
        let mut edges = vec![
            TransitEdge {
                id: EdgeId::new(2),
                from: nodes[1],
                to: nodes[2],
                length_mm: lengths[1],
                allowed_modes: allowed_modes.clone(),
            },
            TransitEdge {
                id: EdgeId::new(1),
                from: nodes[0],
                to: nodes[1],
                length_mm: lengths[0],
                allowed_modes,
            },
        ];
        if !reversed {
            edges.reverse();
        }
        TransitGraphV1::new(1, graph_nodes.to_vec(), edges)
            .unwrap()
            .route(
                NodeId::new(origin),
                NodeId::new(destination),
                &MovementProfile::new(mode_name, speed).unwrap(),
                ticks_per_second,
            )
            .unwrap()
    }

    fn context(route: RoutePlan) -> TransitContext {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let service = flow
            .create_work(owner, SimDuration::from_ticks(1), "service", ())
            .unwrap();
        let acquire = FlowAcquireCommand {
            resource,
            owner,
            work: Some(service),
            at: SimTime::from_ticks(0),
            priority_level: 0,
            deadline: None,
            scheduler_priority: 0,
            timed: true,
            can_preempt: false,
            preemptible: None,
        };
        TransitContext::new(&flow, route, acquire, SimTime::from_ticks(0)).unwrap()
    }

    #[test]
    fn canonical_graph_order_and_purpose_identity_are_stable() {
        let first = context(route_graph([2, 3], false, 1));
        let reordered = context(route_graph([2, 3], true, 1));
        let a = RouteReceipt::from_context(&first, &route_metadata("patient-transfer")).unwrap();
        let b =
            RouteReceipt::from_context(&reordered, &route_metadata("patient-transfer")).unwrap();
        assert_eq!(a.digest(), b.digest());
        assert_eq!(
            a.digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "6e2de4e99a98adaadfb78a64e8a0d438c6036f6c43c7c5b953238893532d5425"
        );
        assert!(a
            .validate_context(&reordered, &route_metadata("patient-transfer"))
            .is_ok());
        let other_purpose =
            RouteReceipt::from_context(&reordered, &route_metadata("staff-transfer")).unwrap();
        assert_ne!(a.digest(), other_purpose.digest());
        assert_eq!(
            a.validate_context(&reordered, &route_metadata("staff-transfer")),
            Err(RouteReceiptError::RouteMismatch)
        );
    }

    #[test]
    fn actual_context_profile_and_topology_mismatches_reject() {
        let expected = context(route_graph([2, 3], false, 1));
        let receipt =
            RouteReceipt::from_context(&expected, &route_metadata("patient-transfer")).unwrap();
        let changed_topology = context(route_graph([2, 4], false, 1));
        let changed_profile = context(route_graph([2, 3], false, 2));
        let changed_origin = context(route_graph_at([2, 3], false, 1, 1, 2, 3, "walk"));
        let changed_destination = context(route_graph_at([2, 3], false, 1, 1, 1, 2, "walk"));
        let changed_tick_rate = context(route_graph_at([2, 3], false, 1, 2, 1, 3, "walk"));
        let changed_mode = context(route_graph_at([2, 3], false, 1, 1, 1, 3, "roll"));
        assert_eq!(
            receipt.validate_context(&changed_topology, &route_metadata("patient-transfer")),
            Err(RouteReceiptError::RouteMismatch)
        );
        assert_eq!(
            receipt.validate_context(&changed_profile, &route_metadata("patient-transfer")),
            Err(RouteReceiptError::RouteMismatch)
        );
        assert_eq!(
            receipt.validate_context(&changed_origin, &route_metadata("patient-transfer")),
            Err(RouteReceiptError::RouteMismatch)
        );
        assert_eq!(
            receipt.validate_context(&changed_destination, &route_metadata("patient-transfer")),
            Err(RouteReceiptError::RouteMismatch)
        );
        assert_eq!(
            receipt.validate_context(&changed_tick_rate, &route_metadata("patient-transfer")),
            Err(RouteReceiptError::RouteMismatch)
        );
        assert_eq!(
            receipt.validate_context(&changed_mode, &route_metadata("patient-transfer")),
            Err(RouteReceiptError::RouteMismatch)
        );
    }

    #[test]
    fn sensor_only_distance_cannot_create_execution_receipt() {
        let actual = context(route_graph([2, 3], false, 1));
        assert_eq!(
            RouteReceipt::from_context(
                &actual,
                &RouteMetadata::v1(
                    "patient-transfer",
                    DistanceProvenance::SensorObservationOnly
                ),
            ),
            Err(RouteReceiptError::SensorObservationOnly)
        );
    }

    #[test]
    fn trip_purpose_rejects_noncanonical_values() {
        let actual = context(route_graph([2, 3], false, 1));
        for purpose in [
            "",
            " patient-transfer",
            "patient-transfer ",
            "patient\ntransfer",
        ] {
            assert_eq!(
                RouteReceipt::from_context(&actual, &route_metadata(purpose),),
                Err(RouteReceiptError::InvalidTripPurpose)
            );
        }
        let too_long = "x".repeat(1025);
        assert_eq!(
            RouteReceipt::from_context(&actual, &route_metadata(&too_long)),
            Err(RouteReceiptError::InvalidTripPurpose)
        );
        let max_utf8 = "é".repeat(512);
        assert!(RouteReceipt::from_context(&actual, &route_metadata(&max_utf8),).is_ok());
        let too_long_utf8 = "é".repeat(513);
        assert_eq!(
            RouteReceipt::from_context(&actual, &route_metadata(&too_long_utf8),),
            Err(RouteReceiptError::InvalidTripPurpose)
        );
    }

    #[test]
    fn actual_transit_carrier_keeps_route_receipt_across_pause_and_resume() {
        let mut flow = FlowRuntime::new();
        register_transit_context(&mut flow, "receipt-transit", KIND).unwrap();
        let owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let service = flow
            .create_work(owner, SimDuration::from_ticks(1), "service", ())
            .unwrap();
        let acquire = FlowAcquireCommand {
            resource,
            owner,
            work: Some(service),
            at: SimTime::from_ticks(0),
            priority_level: 0,
            deadline: None,
            scheduler_priority: 0,
            timed: true,
            can_preempt: false,
            preemptible: None,
        };
        let route = route_graph([2, 3], false, 1);
        let initial = TransitContext::new(&flow, route, acquire, SimTime::from_ticks(0)).unwrap();
        let carrier = flow
            .create_actor_domain_context(carrier_actor, "receipt-transit", KIND, initial)
            .unwrap();
        let receipt = RouteReceipt::from_context(
            flow.work_context::<TransitContext>(carrier).unwrap(),
            &route_metadata("patient-transfer"),
        )
        .unwrap();

        schedule_transit_start(&mut flow, carrier, KIND, SimTime::from_ticks(0), 0).unwrap();
        schedule_transit_control(
            &mut flow,
            carrier,
            KIND,
            FlowDomainControl::Pause,
            SimTime::from_ticks(1),
            0,
        )
        .unwrap();
        assert!(flow.step().unwrap().unwrap().error.is_none()); // start at 0
        assert!(flow.step().unwrap().unwrap().error.is_none()); // pause at 1
        let paused = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(paused.phase(), TransitPhase::Paused);
        let paused_progress = paused.progress_at(flow.now()).unwrap();
        assert_eq!(paused_progress.useful_elapsed.ticks(), 1);
        assert_eq!(paused_progress.remaining.ticks(), 4);
        assert!(receipt
            .validate_context(paused, &route_metadata("patient-transfer"))
            .is_ok());

        schedule_transit_control(
            &mut flow,
            carrier,
            KIND,
            FlowDomainControl::Resume,
            SimTime::from_ticks(3),
            0,
        )
        .unwrap();
        assert!(flow.step().unwrap().unwrap().error.is_none()); // old edge event while paused
        assert!(flow.step().unwrap().unwrap().error.is_none()); // resume at 3
        let resumed = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(resumed.phase(), TransitPhase::Moving);
        let resumed_progress = resumed.progress_at(flow.now()).unwrap();
        assert_eq!(resumed_progress.useful_elapsed.ticks(), 1);
        assert_eq!(resumed_progress.remaining.ticks(), 4);
        assert_eq!(
            receipt.validate_context(resumed, &route_metadata("patient-transfer")),
            Ok(())
        );
        assert!(flow.step().unwrap().unwrap().error.is_none()); // first segment completes at 4
        let arrival_dispatch = flow.step().unwrap().unwrap(); // final segment completes at 7
        assert!(arrival_dispatch.error.is_none());
        assert!(matches!(
            arrival_dispatch.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(admissions)] if admissions.len() == 1
        ));
        assert_eq!(flow.now().ticks(), 7);
        let arrived = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(arrived.phase(), TransitPhase::Arrived);
        let completed_progress = arrived.progress_at(flow.now()).unwrap();
        assert_eq!(completed_progress.useful_elapsed.ticks(), 5);
        assert_eq!(completed_progress.remaining.ticks(), 0);
        assert!(receipt
            .validate_context(arrived, &route_metadata("patient-transfer"))
            .is_ok());
        let request_id = flow.work(service).unwrap().request.unwrap();
        let request = flow.request(request_id).unwrap();
        assert_eq!(request.work, Some(service));
        assert_eq!(request.resource, resource);
        assert_eq!(request.submitted_at, SimTime::from_ticks(7));
    }
}

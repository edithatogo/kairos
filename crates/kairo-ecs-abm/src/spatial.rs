//! Deterministic, integer-only transit geometry and route planning.

use kairo_ecs_types::SimDuration;
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::num::NonZeroU64;

const GRAPH_TAG: &[u8] = b"KAIROS-TRANSIT-GRAPH\0";
const MILLIMETRES_TAG: &[u8] = b"mm\0";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(u64);

impl NodeId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EdgeId(u64);

impl EdgeId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MovementModeId(String);

impl MovementModeId {
    pub fn new(value: &str) -> Result<Self, TransitError> {
        if value.is_empty()
            || value.len() > 1024
            || value.chars().any(char::is_control)
            || value.trim() != value
        {
            return Err(TransitError::InvalidMovementMode);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitEdge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub length_mm: u64,
    pub allowed_modes: Vec<MovementModeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementProfile {
    mode: MovementModeId,
    speed_mm_per_second: NonZeroU64,
}

impl MovementProfile {
    pub fn new(mode: &str, speed_mm_per_second: u64) -> Result<Self, TransitError> {
        let mode = MovementModeId::new(mode)?;
        let speed_mm_per_second =
            NonZeroU64::new(speed_mm_per_second).ok_or(TransitError::InvalidSpeed)?;
        Ok(Self {
            mode,
            speed_mm_per_second,
        })
    }

    pub fn mode(&self) -> &MovementModeId {
        &self.mode
    }

    pub fn speed_mm_per_second(&self) -> NonZeroU64 {
        self.speed_mm_per_second
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitError {
    UnsupportedVersion,
    InvalidGraph,
    InvalidMovementMode,
    InvalidSpeed,
    InvalidTickRate,
    UnknownNode,
    Unreachable,
    Overflow,
    InvalidProgress,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitGraphV1 {
    version: u32,
    nodes: Vec<NodeId>,
    edges: Vec<TransitEdge>,
    canonical_bytes: Vec<u8>,
}

impl TransitGraphV1 {
    pub fn new(
        version: u32,
        mut nodes: Vec<NodeId>,
        mut edges: Vec<TransitEdge>,
    ) -> Result<Self, TransitError> {
        if version != 1 {
            return Err(TransitError::UnsupportedVersion);
        }

        nodes.sort_unstable();
        if nodes.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(TransitError::InvalidGraph);
        }
        let node_set: BTreeSet<_> = nodes.iter().copied().collect();

        edges.sort_unstable_by_key(|edge| edge.id);
        if edges.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(TransitError::InvalidGraph);
        }
        for edge in &mut edges {
            if !node_set.contains(&edge.from) || !node_set.contains(&edge.to) {
                return Err(TransitError::InvalidGraph);
            }
            if edge.allowed_modes.is_empty() {
                return Err(TransitError::InvalidGraph);
            }
            edge.allowed_modes.sort_unstable();
            if edge.allowed_modes.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(TransitError::InvalidGraph);
            }
        }

        let canonical_bytes = canonical_bytes(version, &nodes, &edges)?;
        Ok(Self {
            version,
            nodes,
            edges,
            canonical_bytes,
        })
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.canonical_bytes.clone()
    }

    /// Borrow the canonical graph identity without allocating a copy.
    #[doc(hidden)]
    pub fn canonical_bytes_ref(&self) -> &[u8] {
        &self.canonical_bytes
    }

    pub fn route(
        &self,
        origin: NodeId,
        destination: NodeId,
        profile: &MovementProfile,
        ticks_per_second: u64,
    ) -> Result<RoutePlan, TransitError> {
        if ticks_per_second == 0 {
            return Err(TransitError::InvalidTickRate);
        }
        if self.nodes.binary_search(&origin).is_err()
            || self.nodes.binary_search(&destination).is_err()
        {
            return Err(TransitError::UnknownNode);
        }

        let mut adjacency: BTreeMap<NodeId, Vec<&TransitEdge>> = BTreeMap::new();
        for edge in &self.edges {
            if edge.allowed_modes.binary_search(&profile.mode).is_ok() {
                adjacency.entry(edge.from).or_default().push(edge);
            }
        }
        for outgoing in adjacency.values_mut() {
            outgoing.sort_unstable_by_key(|edge| edge.id);
        }

        let winning_path = if origin == destination {
            Vec::new()
        } else {
            let mut pending = BinaryHeap::new();
            let initial = PathCandidate {
                distance_mm: 0,
                edge_ids: Vec::new(),
                nodes: vec![origin],
                edges: Vec::new(),
            };
            let mut best = BTreeMap::new();
            best.insert(origin, initial.clone());
            pending.push(Reverse(initial));
            let mut winner = None;
            while let Some(Reverse(candidate)) = pending.pop() {
                let last = *candidate.nodes.last().ok_or(TransitError::InvalidGraph)?;
                // A label dominates every other route to this node under
                // (distance, hops, edge IDs): appending the same suffix keeps
                // that order. Any route returning to an earlier node contains
                // a nonnegative cycle and is dominated by removing it (lower
                // distance, or equal distance with fewer hops).
                if best.get(&last) != Some(&candidate) {
                    continue;
                }
                if last == destination {
                    winner = Some(candidate.edges);
                    break;
                }
                if let Some(outgoing) = adjacency.get(&last) {
                    for edge in outgoing {
                        // A globally preferred route is simple: revisiting a
                        // node can only add distance or, for a zero cycle,
                        // preserve distance while adding hops.
                        if candidate.nodes.contains(&edge.to) {
                            continue;
                        }
                        let distance_mm = candidate
                            .distance_mm
                            .checked_add(u128::from(edge.length_mm))
                            .ok_or(TransitError::Overflow)?;
                        let mut next = candidate.clone();
                        next.distance_mm = distance_mm;
                        next.edge_ids.push(edge.id);
                        next.nodes.push(edge.to);
                        next.edges.push((*edge).clone());
                        let improves = match best.get(&edge.to) {
                            None => true,
                            Some(known) => next.cmp(known) == Ordering::Less,
                        };
                        if improves {
                            best.insert(edge.to, next.clone());
                            pending.push(Reverse(next));
                        }
                    }
                }
            }
            winner.ok_or(TransitError::Unreachable)?
        };

        let distance_mm = winning_path.iter().try_fold(0_u128, |total, edge| {
            total
                .checked_add(u128::from(edge.length_mm))
                .ok_or(TransitError::Overflow)
        })?;
        let speed = u128::from(profile.speed_mm_per_second.get());
        let ticks_per_second_u128 = u128::from(ticks_per_second);
        let duration_ticks = ceil_ticks(distance_mm, ticks_per_second_u128, speed)?;

        let mut segments = Vec::with_capacity(winning_path.len());
        let mut cumulative_mm = 0_u128;
        let mut prior_ticks = 0_u128;
        for edge in &winning_path {
            cumulative_mm = cumulative_mm
                .checked_add(u128::from(edge.length_mm))
                .ok_or(TransitError::Overflow)?;
            let end_ticks = ceil_ticks(cumulative_mm, ticks_per_second_u128, speed)?;
            if end_ticks < prior_ticks {
                return Err(TransitError::InvalidProgress);
            }
            segments.push(RouteSegment {
                edge_id: edge.id,
                from: edge.from,
                to: edge.to,
                length_mm: edge.length_mm,
                start_offset: SimDuration::from_ticks(prior_ticks),
                end_offset: SimDuration::from_ticks(end_ticks),
            });
            prior_ticks = end_ticks;
        }
        if prior_ticks != duration_ticks {
            return Err(TransitError::InvalidProgress);
        }

        Ok(RoutePlan {
            origin,
            destination,
            segments,
            distance_mm,
            duration: SimDuration::from_ticks(duration_ticks),
            profile: profile.clone(),
            ticks_per_second,
            graph_version: self.version,
            graph_canonical_bytes: self.canonical_bytes.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PathCandidate {
    distance_mm: u128,
    edge_ids: Vec<EdgeId>,
    nodes: Vec<NodeId>,
    edges: Vec<TransitEdge>,
}

impl Ord for PathCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.distance_mm, &self.edge_ids.len(), &self.edge_ids).cmp(&(
            &other.distance_mm,
            &other.edge_ids.len(),
            &other.edge_ids,
        ))
    }
}

impl PartialOrd for PathCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteSegment {
    edge_id: EdgeId,
    from: NodeId,
    to: NodeId,
    length_mm: u64,
    start_offset: SimDuration,
    end_offset: SimDuration,
}

impl RouteSegment {
    pub fn edge_id(&self) -> EdgeId {
        self.edge_id
    }

    pub fn from(&self) -> NodeId {
        self.from
    }

    pub fn to(&self) -> NodeId {
        self.to
    }

    pub fn length_mm(&self) -> u64 {
        self.length_mm
    }

    pub fn start_offset(&self) -> SimDuration {
        self.start_offset
    }

    pub fn end_offset(&self) -> SimDuration {
        self.end_offset
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutePlan {
    origin: NodeId,
    destination: NodeId,
    segments: Vec<RouteSegment>,
    distance_mm: u128,
    duration: SimDuration,
    profile: MovementProfile,
    ticks_per_second: u64,
    graph_version: u32,
    graph_canonical_bytes: Vec<u8>,
}

impl RoutePlan {
    pub fn origin(&self) -> NodeId {
        self.origin
    }

    pub fn destination(&self) -> NodeId {
        self.destination
    }

    pub fn segments(&self) -> &[RouteSegment] {
        &self.segments
    }

    pub fn distance_mm(&self) -> u128 {
        self.distance_mm
    }

    pub fn duration(&self) -> SimDuration {
        self.duration
    }

    pub fn profile(&self) -> &MovementProfile {
        &self.profile
    }

    pub fn ticks_per_second(&self) -> u64 {
        self.ticks_per_second
    }

    pub fn graph_version(&self) -> u32 {
        self.graph_version
    }

    pub fn graph_canonical_bytes(&self) -> &[u8] {
        &self.graph_canonical_bytes
    }

    /// Captures a bounded native image of this immutable route.
    #[doc(hidden)]
    pub fn checkpoint_image_v1(
        &self,
        limits: &RoutePlanImageLimitsV1,
    ) -> Result<RoutePlanImageV1, RouteCheckpointError> {
        limits.check_parts(
            self.segments.len(),
            self.graph_canonical_bytes.len(),
            self.profile.mode.as_str().len(),
        )?;
        validate_route_plan(self).map_err(|_| RouteCheckpointError::InvalidPlan)?;
        Ok(RoutePlanImageV1 {
            schema_version: 1,
            origin: self.origin.value(),
            destination: self.destination.value(),
            segments: self
                .segments
                .iter()
                .map(|segment| RouteSegmentImageV1 {
                    edge_id: segment.edge_id.value(),
                    from: segment.from.value(),
                    to: segment.to.value(),
                    length_mm: segment.length_mm,
                    start_offset_ticks: segment.start_offset.ticks(),
                    end_offset_ticks: segment.end_offset.ticks(),
                })
                .collect(),
            distance_mm: self.distance_mm,
            duration_ticks: self.duration.ticks(),
            movement_mode: self.profile.mode.as_str().to_owned(),
            speed_mm_per_second: self.profile.speed_mm_per_second.get(),
            ticks_per_second: self.ticks_per_second,
            graph_version: self.graph_version,
            graph_canonical_bytes: self.graph_canonical_bytes.clone(),
        })
    }

    /// Validates a route image against the supplied immutable graph and restores
    /// the exact deterministic plan represented by that image.
    #[doc(hidden)]
    pub fn restore_image_v1(
        image: &RoutePlanImageV1,
        graph: &TransitGraphV1,
        limits: &RoutePlanImageLimitsV1,
    ) -> Result<Self, RouteCheckpointError> {
        image.validate_limits(limits)?;
        if image.schema_version != 1 {
            return Err(RouteCheckpointError::UnsupportedVersion);
        }
        let profile = MovementProfile::new(&image.movement_mode, image.speed_mm_per_second)
            .map_err(|_| RouteCheckpointError::InvalidPlan)?;
        let route = graph
            .route(
                NodeId::new(image.origin),
                NodeId::new(image.destination),
                &profile,
                image.ticks_per_second,
            )
            .map_err(|_| RouteCheckpointError::InvalidGraph)?;
        if !image.matches_route(&route) {
            return Err(RouteCheckpointError::InvalidPlan);
        }
        Ok(route)
    }
}

/// Bounded allocation limits for the native route image.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoutePlanImageLimitsV1 {
    pub max_segments: usize,
    pub max_graph_bytes: usize,
    pub max_mode_bytes: usize,
    pub max_total_bytes: usize,
}

impl RoutePlanImageLimitsV1 {
    pub const fn new(
        max_segments: usize,
        max_graph_bytes: usize,
        max_mode_bytes: usize,
        max_total_bytes: usize,
    ) -> Self {
        Self {
            max_segments,
            max_graph_bytes,
            max_mode_bytes,
            max_total_bytes,
        }
    }

    pub(crate) fn check_parts(
        &self,
        segment_count: usize,
        graph_bytes: usize,
        mode_bytes: usize,
    ) -> Result<(), RouteCheckpointError> {
        let total = segment_count
            .checked_mul(64)
            .and_then(|bytes| bytes.checked_add(graph_bytes))
            .and_then(|bytes| bytes.checked_add(mode_bytes))
            .ok_or(RouteCheckpointError::LimitExceeded)?;
        if segment_count > self.max_segments
            || graph_bytes > self.max_graph_bytes
            || mode_bytes > self.max_mode_bytes
            || total > self.max_total_bytes
        {
            return Err(RouteCheckpointError::LimitExceeded);
        }
        Ok(())
    }
}

/// Native owner-defined immutable route image, schema version 1.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutePlanImageV1 {
    pub schema_version: u32,
    pub origin: u64,
    pub destination: u64,
    pub segments: Vec<RouteSegmentImageV1>,
    pub distance_mm: u128,
    pub duration_ticks: u128,
    pub movement_mode: String,
    pub speed_mm_per_second: u64,
    pub ticks_per_second: u64,
    pub graph_version: u32,
    pub graph_canonical_bytes: Vec<u8>,
}

#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteSegmentImageV1 {
    pub edge_id: u64,
    pub from: u64,
    pub to: u64,
    pub length_mm: u64,
    pub start_offset_ticks: u128,
    pub end_offset_ticks: u128,
}

/// Errors for the experimental route-image API; public transit errors stay stable.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RouteCheckpointError {
    UnsupportedVersion,
    LimitExceeded,
    InvalidPlan,
    InvalidGraph,
    InvalidProgress,
}

impl RoutePlanImageV1 {
    pub(crate) fn validate_limits(
        &self,
        limits: &RoutePlanImageLimitsV1,
    ) -> Result<(), RouteCheckpointError> {
        limits.check_parts(
            self.segments.len(),
            self.graph_canonical_bytes.len(),
            self.movement_mode.len(),
        )
    }

    fn matches_route(&self, route: &RoutePlan) -> bool {
        self.origin == route.origin.value()
            && self.destination == route.destination.value()
            && self.distance_mm == route.distance_mm
            && self.duration_ticks == route.duration.ticks()
            && self.movement_mode == route.profile.mode.as_str()
            && self.speed_mm_per_second == route.profile.speed_mm_per_second.get()
            && self.ticks_per_second == route.ticks_per_second
            && self.graph_version == route.graph_version
            && self.graph_canonical_bytes == route.graph_canonical_bytes
            && self.segments.len() == route.segments.len()
            && self
                .segments
                .iter()
                .zip(&route.segments)
                .all(|(image, segment)| {
                    image.edge_id == segment.edge_id.value()
                        && image.from == segment.from.value()
                        && image.to == segment.to.value()
                        && image.length_mm == segment.length_mm
                        && image.start_offset_ticks == segment.start_offset.ticks()
                        && image.end_offset_ticks == segment.end_offset.ticks()
                })
    }
}

fn validate_route_plan(route: &RoutePlan) -> Result<(), TransitError> {
    if route.ticks_per_second == 0 || route.profile.speed_mm_per_second.get() == 0 {
        return Err(TransitError::InvalidProgress);
    }
    let mut distance = 0_u128;
    let mut prior_end = SimDuration::ZERO;
    let mut prior_node = route.origin;
    for segment in &route.segments {
        if segment.from != prior_node
            || segment.start_offset != prior_end
            || segment.end_offset < segment.start_offset
        {
            return Err(TransitError::InvalidProgress);
        }
        distance = distance
            .checked_add(u128::from(segment.length_mm))
            .ok_or(TransitError::Overflow)?;
        prior_end = segment.end_offset;
        prior_node = segment.to;
    }
    if prior_node != route.destination
        || distance != route.distance_mm
        || prior_end != route.duration
        || (route.segments.is_empty() && route.origin != route.destination)
    {
        return Err(TransitError::InvalidProgress);
    }
    Ok(())
}

fn ceil_ticks(
    distance_mm: u128,
    ticks_per_second: u128,
    speed: u128,
) -> Result<u128, TransitError> {
    let numerator = distance_mm
        .checked_mul(ticks_per_second)
        .ok_or(TransitError::Overflow)?;
    let quotient = numerator / speed;
    if numerator % speed == 0 {
        Ok(quotient)
    } else {
        quotient.checked_add(1).ok_or(TransitError::Overflow)
    }
}

fn canonical_bytes(
    version: u32,
    nodes: &[NodeId],
    edges: &[TransitEdge],
) -> Result<Vec<u8>, TransitError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(GRAPH_TAG);
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(MILLIMETRES_TAG);
    put_len(&mut bytes, nodes.len())?;
    for node in nodes {
        bytes.extend_from_slice(&node.0.to_le_bytes());
    }
    put_len(&mut bytes, edges.len())?;
    for edge in edges {
        bytes.extend_from_slice(&edge.id.0.to_le_bytes());
        bytes.extend_from_slice(&edge.from.0.to_le_bytes());
        bytes.extend_from_slice(&edge.to.0.to_le_bytes());
        bytes.extend_from_slice(&edge.length_mm.to_le_bytes());
        put_len(&mut bytes, edge.allowed_modes.len())?;
        for mode in &edge.allowed_modes {
            put_len(&mut bytes, mode.0.len())?;
            bytes.extend_from_slice(mode.0.as_bytes());
        }
    }
    Ok(bytes)
}

fn put_len(bytes: &mut Vec<u8>, len: usize) -> Result<(), TransitError> {
    let value = u64::try_from(len).map_err(|_| TransitError::Overflow)?;
    bytes.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

/// In-memory route cursor used by the Flow adapter to retain actual transit
/// movement independently of service-work restart state.
#[allow(dead_code)] // The Flow carrier consumes this private cursor in the dispatch leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TransitProgressState {
    route: RoutePlan,
    segment_index: usize,
    elapsed_in_segment: SimDuration,
}

/// Cloneable interruption checkpoint for the in-memory transit cursor.
#[allow(dead_code)] // The Flow carrier consumes this private checkpoint in the dispatch leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TransitProgressCheckpoint {
    route: RoutePlanImageV1,
    segment_index: usize,
    elapsed_in_segment: SimDuration,
}

#[allow(dead_code)] // Kept crate-private until the reviewed Flow dispatch adapter uses it.
impl TransitProgressState {
    pub(crate) fn new(route: RoutePlan) -> Result<Self, TransitError> {
        let mut state = Self {
            route,
            segment_index: 0,
            elapsed_in_segment: SimDuration::ZERO,
        };
        state.validate_route()?;
        state.skip_zero_duration_segments()?;
        Ok(state)
    }

    pub(crate) fn current_edge_id(&self) -> Option<EdgeId> {
        self.route
            .segments
            .get(self.segment_index)
            .map(|segment| segment.edge_id)
    }

    pub(crate) fn segment_index(&self) -> usize {
        self.segment_index
    }

    pub(crate) fn current_segment_remaining(&self) -> Result<SimDuration, TransitError> {
        self.validate_cursor()?;
        let Some(segment) = self.route.segments.get(self.segment_index) else {
            return Ok(SimDuration::ZERO);
        };
        let duration = segment
            .end_offset
            .checked_sub(segment.start_offset)
            .ok_or(TransitError::InvalidProgress)?;
        duration
            .checked_sub(self.elapsed_in_segment)
            .ok_or(TransitError::InvalidProgress)
    }

    pub(crate) fn elapsed_in_segment(&self) -> SimDuration {
        self.elapsed_in_segment
    }

    pub(crate) fn remaining(&self) -> Result<SimDuration, TransitError> {
        self.validate_cursor()?;
        let Some(segment) = self.route.segments.get(self.segment_index) else {
            return Ok(SimDuration::ZERO);
        };
        let segment_duration = segment
            .end_offset
            .checked_sub(segment.start_offset)
            .ok_or(TransitError::InvalidProgress)?;
        let current_remaining = segment_duration
            .checked_sub(self.elapsed_in_segment)
            .ok_or(TransitError::InvalidProgress)?;
        let later_remaining = self
            .route
            .duration
            .checked_sub(segment.end_offset)
            .ok_or(TransitError::InvalidProgress)?;
        current_remaining
            .checked_add(later_remaining)
            .ok_or(TransitError::Overflow)
    }

    pub(crate) fn useful_elapsed(&self) -> Result<SimDuration, TransitError> {
        self.route
            .duration
            .checked_sub(self.remaining()?)
            .ok_or(TransitError::InvalidProgress)
    }

    /// Advance actual movement time. An over-advance fails atomically so a
    /// rejected callback can retry from the same edge and elapsed position.
    pub(crate) fn advance(&mut self, elapsed: SimDuration) -> Result<(), TransitError> {
        self.validate_cursor()?;
        let mut segment_index = self.segment_index;
        let mut elapsed_in_segment = self.elapsed_in_segment;
        let mut to_advance = elapsed;

        loop {
            while let Some(segment) = self.route.segments.get(segment_index) {
                let segment_duration = segment
                    .end_offset
                    .checked_sub(segment.start_offset)
                    .ok_or(TransitError::InvalidProgress)?;
                if segment_duration.ticks() != 0 {
                    break;
                }
                segment_index = segment_index
                    .checked_add(1)
                    .ok_or(TransitError::InvalidProgress)?;
                elapsed_in_segment = SimDuration::ZERO;
            }

            let Some(segment) = self.route.segments.get(segment_index) else {
                if to_advance.ticks() != 0 {
                    return Err(TransitError::InvalidProgress);
                }
                break;
            };
            let segment_duration = segment
                .end_offset
                .checked_sub(segment.start_offset)
                .ok_or(TransitError::InvalidProgress)?;
            let remaining_in_segment = segment_duration
                .checked_sub(elapsed_in_segment)
                .ok_or(TransitError::InvalidProgress)?;

            if to_advance < remaining_in_segment {
                elapsed_in_segment = elapsed_in_segment
                    .checked_add(to_advance)
                    .ok_or(TransitError::Overflow)?;
                break;
            }

            to_advance = to_advance
                .checked_sub(remaining_in_segment)
                .ok_or(TransitError::InvalidProgress)?;
            segment_index = segment_index
                .checked_add(1)
                .ok_or(TransitError::InvalidProgress)?;
            elapsed_in_segment = SimDuration::ZERO;
        }

        let mut candidate = self.clone();
        candidate.segment_index = segment_index;
        candidate.elapsed_in_segment = elapsed_in_segment;
        candidate.validate_cursor()?;
        self.segment_index = candidate.segment_index;
        self.elapsed_in_segment = candidate.elapsed_in_segment;
        Ok(())
    }

    pub(crate) fn checkpoint(
        &self,
        limits: &RoutePlanImageLimitsV1,
    ) -> Result<TransitProgressCheckpoint, RouteCheckpointError> {
        self.validate_cursor()
            .map_err(|_| RouteCheckpointError::InvalidProgress)?;
        Ok(TransitProgressCheckpoint {
            route: self.route.checkpoint_image_v1(limits)?,
            segment_index: self.segment_index,
            elapsed_in_segment: self.elapsed_in_segment,
        })
    }

    pub(crate) fn checkpoint_cursor(&self) -> Result<(usize, SimDuration), RouteCheckpointError> {
        self.validate_cursor()
            .map_err(|_| RouteCheckpointError::InvalidProgress)?;
        Ok((self.segment_index, self.elapsed_in_segment))
    }

    pub(crate) fn is_initial_cursor(&self) -> bool {
        self.segment_index
            == self
                .route
                .segments
                .iter()
                .position(|segment| segment.end_offset > segment.start_offset)
                .unwrap_or(self.route.segments.len())
            && self.elapsed_in_segment == SimDuration::ZERO
    }

    pub(crate) fn is_complete_cursor(&self) -> bool {
        self.segment_index == self.route.segments.len()
            && self.elapsed_in_segment == SimDuration::ZERO
    }

    pub(crate) fn uses_route(&self, route: &RoutePlan) -> bool {
        &self.route == route
    }

    pub(crate) fn restore_at_cursor(
        route: RoutePlan,
        segment_index: usize,
        elapsed_in_segment: SimDuration,
    ) -> Result<Self, RouteCheckpointError> {
        let state = Self {
            route,
            segment_index,
            elapsed_in_segment,
        };
        state
            .validate_cursor()
            .map_err(|_| RouteCheckpointError::InvalidProgress)?;
        Ok(state)
    }

    pub(crate) fn restore(
        checkpoint: TransitProgressCheckpoint,
        graph: &TransitGraphV1,
        limits: &RoutePlanImageLimitsV1,
    ) -> Result<Self, RouteCheckpointError> {
        let state = Self {
            route: RoutePlan::restore_image_v1(&checkpoint.route, graph, limits)?,
            segment_index: checkpoint.segment_index,
            elapsed_in_segment: checkpoint.elapsed_in_segment,
        };
        state
            .validate_cursor()
            .map_err(|_| RouteCheckpointError::InvalidProgress)?;
        Ok(state)
    }

    fn skip_zero_duration_segments(&mut self) -> Result<(), TransitError> {
        while let Some(segment) = self.route.segments.get(self.segment_index) {
            let segment_duration = segment
                .end_offset
                .checked_sub(segment.start_offset)
                .ok_or(TransitError::InvalidProgress)?;
            if segment_duration.ticks() != 0 {
                break;
            }
            self.segment_index = self
                .segment_index
                .checked_add(1)
                .ok_or(TransitError::InvalidProgress)?;
        }
        self.validate_cursor()
    }

    fn validate_cursor(&self) -> Result<(), TransitError> {
        self.validate_route()?;
        if self.segment_index > self.route.segments.len() {
            return Err(TransitError::InvalidProgress);
        }
        let Some(segment) = self.route.segments.get(self.segment_index) else {
            return if self.elapsed_in_segment.ticks() == 0 {
                Ok(())
            } else {
                Err(TransitError::InvalidProgress)
            };
        };
        let segment_duration = segment
            .end_offset
            .checked_sub(segment.start_offset)
            .ok_or(TransitError::InvalidProgress)?;
        if segment_duration.ticks() == 0 || self.elapsed_in_segment >= segment_duration {
            return Err(TransitError::InvalidProgress);
        }
        Ok(())
    }

    fn validate_route(&self) -> Result<(), TransitError> {
        let segments = &self.route.segments;
        let Some(first) = segments.first() else {
            return if self.route.duration.ticks() == 0 {
                Ok(())
            } else {
                Err(TransitError::InvalidProgress)
            };
        };
        if first.start_offset.ticks() != 0 {
            return Err(TransitError::InvalidProgress);
        }
        let mut prior_end = SimDuration::ZERO;
        for segment in segments {
            if segment.start_offset != prior_end || segment.end_offset < segment.start_offset {
                return Err(TransitError::InvalidProgress);
            }
            prior_end = segment.end_offset;
        }
        if prior_end != self.route.duration {
            return Err(TransitError::InvalidProgress);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EdgeId, MovementModeId, MovementProfile, NodeId, RouteCheckpointError, RoutePlan,
        RoutePlanImageLimitsV1, SimDuration, TransitEdge, TransitError, TransitGraphV1,
        TransitProgressState,
    };

    fn image_limits() -> RoutePlanImageLimitsV1 {
        RoutePlanImageLimitsV1::new(128, 16_384, 1_024, 32_768)
    }

    fn linear_graph(lengths_mm: &[u64]) -> TransitGraphV1 {
        let nodes: Vec<_> = (0..=lengths_mm.len())
            .map(|value| NodeId::new(u64::try_from(value).unwrap()))
            .collect();
        let mode = MovementModeId::new("walk").unwrap();
        let edges = lengths_mm
            .iter()
            .enumerate()
            .map(|(index, length_mm)| TransitEdge {
                id: EdgeId::new(u64::try_from(index + 1).unwrap()),
                from: nodes[index],
                to: nodes[index + 1],
                length_mm: *length_mm,
                allowed_modes: vec![mode.clone()],
            })
            .collect();
        TransitGraphV1::new(1, nodes, edges).unwrap()
    }

    #[test]
    fn canonical_graph_identity_can_be_borrowed_without_changing_owned_api() {
        let graph = linear_graph(&[5, 3]);
        assert_eq!(
            graph.canonical_bytes_ref(),
            graph.canonical_bytes().as_slice()
        );
    }

    fn linear_route(
        lengths_mm: &[u64],
        speed_mm_per_second: u64,
        ticks_per_second: u64,
    ) -> RoutePlan {
        let graph = linear_graph(lengths_mm);
        graph
            .route(
                NodeId::new(0),
                NodeId::new(u64::try_from(lengths_mm.len()).unwrap()),
                &MovementProfile::new("walk", speed_mm_per_second).unwrap(),
                ticks_per_second,
            )
            .unwrap()
    }

    #[test]
    fn transit_progress_checkpoint_restores_edge_elapsed_and_remaining_duration() {
        let graph = linear_graph(&[5, 3, 4]);
        let route = graph
            .route(
                NodeId::new(0),
                NodeId::new(3),
                &MovementProfile::new("walk", 2).unwrap(),
                3,
            )
            .unwrap();
        let mut progress = TransitProgressState::new(route.clone()).unwrap();
        progress.advance(SimDuration::from_ticks(10)).unwrap();

        assert_eq!(progress.current_edge_id(), Some(EdgeId::new(2)));
        assert_eq!(progress.elapsed_in_segment(), SimDuration::from_ticks(2));
        assert_eq!(
            progress.useful_elapsed().unwrap(),
            SimDuration::from_ticks(10)
        );
        assert_eq!(progress.remaining().unwrap(), SimDuration::from_ticks(8));

        let resumed = TransitProgressState::restore(
            progress.checkpoint(&image_limits()).unwrap(),
            &graph,
            &image_limits(),
        )
        .unwrap();
        assert_eq!(resumed, progress);
        assert_eq!(resumed.route, route);
        assert_eq!(resumed.current_edge_id(), Some(EdgeId::new(2)));
        assert_eq!(resumed.remaining().unwrap(), SimDuration::from_ticks(8));

        let mut resumed = resumed;
        resumed.advance(SimDuration::from_ticks(8)).unwrap();
        assert_eq!(resumed.current_edge_id(), None);
        assert_eq!(resumed.remaining().unwrap(), SimDuration::ZERO);
    }

    #[test]
    fn transit_progress_over_advance_rejects_without_losing_checkpoint_state() {
        let mut progress = TransitProgressState::new(linear_route(&[5], 1, 1)).unwrap();
        progress.advance(SimDuration::from_ticks(2)).unwrap();
        let before = progress.clone();

        assert_eq!(
            progress.advance(SimDuration::from_ticks(4)),
            Err(TransitError::InvalidProgress)
        );
        assert_eq!(progress, before);
    }

    #[test]
    fn transit_progress_consumes_zero_duration_edges_at_the_boundary() {
        let mut progress = TransitProgressState::new(linear_route(&[1, 1], u64::MAX, 1)).unwrap();
        assert_eq!(progress.current_edge_id(), Some(EdgeId::new(1)));
        assert_eq!(progress.remaining().unwrap(), SimDuration::from_ticks(1));

        progress.advance(SimDuration::from_ticks(1)).unwrap();
        assert_eq!(progress.current_edge_id(), None);
        assert_eq!(progress.elapsed_in_segment(), SimDuration::ZERO);
        assert_eq!(progress.remaining().unwrap(), SimDuration::ZERO);
    }

    #[test]
    fn transit_progress_rejects_out_of_range_restored_cursor() {
        let progress = TransitProgressState::new(linear_route(&[5], 1, 1)).unwrap();
        let mut checkpoint = progress.checkpoint(&image_limits()).unwrap();
        checkpoint.segment_index = usize::MAX;

        assert_eq!(
            TransitProgressState::restore(checkpoint, &linear_graph(&[5]), &image_limits()),
            Err(RouteCheckpointError::InvalidProgress)
        );
    }

    #[test]
    fn route_image_roundtrips_exact_plan_and_rejects_geometry_changes() {
        let graph = linear_graph(&[5, 3, 4]);
        let profile = MovementProfile::new("walk", 2).unwrap();
        let route = graph
            .route(NodeId::new(0), NodeId::new(3), &profile, 3)
            .unwrap();
        let limits = image_limits();
        let image = route.checkpoint_image_v1(&limits).unwrap();
        let original = image.clone();
        assert_eq!(
            RoutePlan::restore_image_v1(&image, &graph, &limits).unwrap(),
            route
        );

        let mut changed = image.clone();
        changed.segments[1].edge_id += 10;
        assert_eq!(
            RoutePlan::restore_image_v1(&changed, &graph, &limits),
            Err(RouteCheckpointError::InvalidPlan)
        );
        assert_eq!(image, original);

        let changed_graph = linear_graph(&[5, 4, 4]);
        assert_eq!(
            RoutePlan::restore_image_v1(&image, &changed_graph, &limits),
            Err(RouteCheckpointError::InvalidPlan)
        );

        let mut invalid_images = Vec::new();
        let mut candidate = image.clone();
        candidate.schema_version = 2;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.origin = 99;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.destination = 2;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.distance_mm += 1;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.duration_ticks += 1;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.movement_mode = " walk".to_owned();
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.speed_mm_per_second = 0;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.ticks_per_second = 0;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.graph_version = 2;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.graph_canonical_bytes.push(0);
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.segments[0].from = 2;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.segments[0].to = 2;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.segments[0].length_mm += 1;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.segments[0].start_offset_ticks = 1;
        invalid_images.push(candidate);
        let mut candidate = image.clone();
        candidate.segments[0].end_offset_ticks = 0;
        invalid_images.push(candidate);
        for invalid in invalid_images {
            assert!(RoutePlan::restore_image_v1(&invalid, &graph, &limits).is_err());
        }
        assert_eq!(image, original);
    }

    #[test]
    fn route_image_preserves_profile_tie_winner_and_enforces_all_capture_limits() {
        let node = |value| NodeId::new(value);
        let mode = MovementModeId::new("walk").unwrap();
        let edge = |id, from, to, length_mm| TransitEdge {
            id: EdgeId::new(id),
            from: node(from),
            to: node(to),
            length_mm,
            allowed_modes: vec![mode.clone()],
        };
        let graph = TransitGraphV1::new(
            1,
            vec![node(0), node(1), node(2), node(3)],
            vec![
                edge(9, 0, 1, 4),
                edge(3, 0, 2, 4),
                edge(10, 1, 3, 4),
                edge(4, 2, 3, 4),
            ],
        )
        .unwrap();
        let route = graph
            .route(
                node(0),
                node(3),
                &MovementProfile::new("walk", 2).unwrap(),
                5,
            )
            .unwrap();
        assert_eq!(
            route
                .segments()
                .iter()
                .map(|segment| segment.edge_id().value())
                .collect::<Vec<_>>(),
            vec![3, 4]
        );
        let limits = image_limits();
        let image = route.checkpoint_image_v1(&limits).unwrap();
        assert_eq!(
            RoutePlan::restore_image_v1(&image, &graph, &limits).unwrap(),
            route
        );
        let faster = graph
            .route(
                node(0),
                node(3),
                &MovementProfile::new("walk", 4).unwrap(),
                5,
            )
            .unwrap();
        let faster_image = faster.checkpoint_image_v1(&limits).unwrap();
        assert_eq!(
            RoutePlan::restore_image_v1(&faster_image, &graph, &limits).unwrap(),
            faster
        );
        assert_eq!(
            RoutePlan::restore_image_v1(
                &image,
                &graph,
                &RoutePlanImageLimitsV1::new(1, usize::MAX, usize::MAX, usize::MAX)
            ),
            Err(RouteCheckpointError::LimitExceeded)
        );
        assert_eq!(
            route.checkpoint_image_v1(&RoutePlanImageLimitsV1::new(
                usize::MAX,
                1,
                usize::MAX,
                usize::MAX
            )),
            Err(RouteCheckpointError::LimitExceeded)
        );
        assert_eq!(
            route.checkpoint_image_v1(&RoutePlanImageLimitsV1::new(
                usize::MAX,
                usize::MAX,
                1,
                usize::MAX
            )),
            Err(RouteCheckpointError::LimitExceeded)
        );
        assert_eq!(
            route.checkpoint_image_v1(&RoutePlanImageLimitsV1::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                1
            )),
            Err(RouteCheckpointError::LimitExceeded)
        );
    }

    #[test]
    fn progress_image_restores_boundary_zero_tick_and_completed_cursors_directly() {
        let graph = linear_graph(&[5, 0, 3]);
        let route = graph
            .route(
                NodeId::new(0),
                NodeId::new(3),
                &MovementProfile::new("walk", 1).unwrap(),
                1,
            )
            .unwrap();
        let limits = image_limits();
        let mut progress = TransitProgressState::new(route).unwrap();
        progress.advance(SimDuration::from_ticks(5)).unwrap();
        assert_eq!(progress.current_edge_id(), Some(EdgeId::new(3)));
        let resumed =
            TransitProgressState::restore(progress.checkpoint(&limits).unwrap(), &graph, &limits)
                .unwrap();
        assert_eq!(resumed, progress);
        assert_eq!(
            resumed.useful_elapsed().unwrap(),
            SimDuration::from_ticks(5)
        );
        assert_eq!(resumed.remaining().unwrap(), SimDuration::from_ticks(3));
        let mut resumed = resumed;
        resumed.advance(SimDuration::from_ticks(1)).unwrap();
        assert_eq!(resumed.current_edge_id(), Some(EdgeId::new(3)));
        assert_eq!(resumed.elapsed_in_segment(), SimDuration::from_ticks(1));

        resumed.advance(SimDuration::from_ticks(2)).unwrap();
        let completed =
            TransitProgressState::restore(resumed.checkpoint(&limits).unwrap(), &graph, &limits)
                .unwrap();
        assert_eq!(completed.current_edge_id(), None);
        assert_eq!(
            completed.useful_elapsed().unwrap(),
            SimDuration::from_ticks(8)
        );
        assert_eq!(completed.remaining().unwrap(), SimDuration::ZERO);
    }

    #[test]
    fn zero_cycle_does_not_override_a_late_full_sequence_tie_break() {
        let node = |value| NodeId::new(value);
        let mode = MovementModeId::new("walk").unwrap();
        let edge = |id, from, to| TransitEdge {
            id: EdgeId::new(id),
            from: node(from),
            to: node(to),
            length_mm: 0,
            allowed_modes: vec![mode.clone()],
        };
        let graph = TransitGraphV1::new(
            1,
            (0..=6).map(node).collect(),
            vec![
                edge(50, 0, 1),
                edge(90, 1, 2),
                edge(70, 2, 3),
                edge(40, 3, 4),
                edge(4, 4, 6),
                edge(30, 3, 5),
                edge(99, 5, 6),
                edge(0, 1, 0),
            ],
        )
        .unwrap();
        let route = graph
            .route(
                node(0),
                node(6),
                &MovementProfile::new("walk", 1).unwrap(),
                1,
            )
            .unwrap();
        let ids: Vec<_> = route
            .segments()
            .iter()
            .map(|segment| segment.edge_id().value())
            .collect();
        assert_eq!(ids, vec![50, 90, 70, 30, 99]);
    }

    #[test]
    fn dense_zero_cost_layers_choose_the_lexicographic_shortest_route() {
        const LAYERS: u64 = 14;
        let origin = NodeId::new(0);
        let destination = NodeId::new(2 * LAYERS + 1);
        let mut nodes = vec![origin, destination];
        for layer in 0..LAYERS {
            nodes.push(NodeId::new(2 * layer + 1));
            nodes.push(NodeId::new(2 * layer + 2));
        }

        let mode = MovementModeId::new("walk").unwrap();
        let mut edges = Vec::new();
        let mut next_id = 1_u64;
        let mut add_edge = |from, to| {
            edges.push(TransitEdge {
                id: EdgeId::new(next_id),
                from: NodeId::new(from),
                to: NodeId::new(to),
                length_mm: 0,
                allowed_modes: vec![mode.clone()],
            });
            next_id += 1;
        };

        add_edge(origin.value(), 1);
        add_edge(origin.value(), 2);
        for layer in 0..LAYERS - 1 {
            let from = 2 * layer + 1;
            let next = 2 * layer + 3;
            add_edge(from, next);
            add_edge(from, next + 1);
            add_edge(from + 1, next);
            add_edge(from + 1, next + 1);
        }
        add_edge(2 * LAYERS - 1, destination.value());
        add_edge(2 * LAYERS, destination.value());
        drop(add_edge);

        let graph = TransitGraphV1::new(1, nodes, edges).unwrap();
        let route = graph
            .route(
                origin,
                destination,
                &MovementProfile::new("walk", 1).unwrap(),
                1,
            )
            .unwrap();
        let actual: Vec<_> = route
            .segments()
            .iter()
            .map(|segment| segment.edge_id().value())
            .collect();
        let mut expected = vec![1];
        expected.extend((0..LAYERS - 1).map(|layer| 3 + 4 * layer));
        expected.push(next_id - 2);
        assert_eq!(actual, expected);
        assert_eq!(route.segments().len(), usize::try_from(LAYERS + 1).unwrap());
    }
}

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
    route: RoutePlan,
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

    pub(crate) fn checkpoint(&self) -> Result<TransitProgressCheckpoint, TransitError> {
        self.validate_cursor()?;
        Ok(TransitProgressCheckpoint {
            route: self.route.clone(),
            segment_index: self.segment_index,
            elapsed_in_segment: self.elapsed_in_segment,
        })
    }

    pub(crate) fn restore(checkpoint: TransitProgressCheckpoint) -> Result<Self, TransitError> {
        let state = Self {
            route: checkpoint.route,
            segment_index: checkpoint.segment_index,
            elapsed_in_segment: checkpoint.elapsed_in_segment,
        };
        state.validate_cursor()?;
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
        EdgeId, MovementModeId, MovementProfile, NodeId, RoutePlan, SimDuration, TransitEdge,
        TransitError, TransitGraphV1, TransitProgressState,
    };

    fn linear_route(
        lengths_mm: &[u64],
        speed_mm_per_second: u64,
        ticks_per_second: u64,
    ) -> RoutePlan {
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
        TransitGraphV1::new(1, nodes.clone(), edges)
            .unwrap()
            .route(
                nodes[0],
                *nodes.last().unwrap(),
                &MovementProfile::new("walk", speed_mm_per_second).unwrap(),
                ticks_per_second,
            )
            .unwrap()
    }

    #[test]
    fn transit_progress_checkpoint_restores_edge_elapsed_and_remaining_duration() {
        let route = linear_route(&[5, 3, 4], 2, 3);
        let mut progress = TransitProgressState::new(route.clone()).unwrap();
        progress.advance(SimDuration::from_ticks(10)).unwrap();

        assert_eq!(progress.current_edge_id(), Some(EdgeId::new(2)));
        assert_eq!(progress.elapsed_in_segment(), SimDuration::from_ticks(2));
        assert_eq!(
            progress.useful_elapsed().unwrap(),
            SimDuration::from_ticks(10)
        );
        assert_eq!(progress.remaining().unwrap(), SimDuration::from_ticks(8));

        let resumed = TransitProgressState::restore(progress.checkpoint().unwrap()).unwrap();
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
        let mut checkpoint = progress.checkpoint().unwrap();
        checkpoint.segment_index = usize::MAX;

        assert_eq!(
            TransitProgressState::restore(checkpoint),
            Err(TransitError::InvalidProgress)
        );
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

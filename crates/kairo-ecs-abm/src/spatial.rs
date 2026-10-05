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
            pending.push(Reverse(PathCandidate {
                distance_mm: 0,
                edge_ids: Vec::new(),
                nodes: vec![origin],
                edges: Vec::new(),
            }));
            let mut winner = None;
            while let Some(Reverse(candidate)) = pending.pop() {
                let last = *candidate.nodes.last().ok_or(TransitError::InvalidGraph)?;
                if last == destination {
                    winner = Some(candidate.edges);
                    break;
                }
                if let Some(outgoing) = adjacency.get(&last) {
                    for edge in outgoing {
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
                        pending.push(Reverse(next));
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

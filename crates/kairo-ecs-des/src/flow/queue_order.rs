//! Opaque stable tie-break keys for opt-in Flow queue ordering.

use crate::RequestId;
use crate::ResourceId;
use std::collections::BTreeSet;

/// Maximum byte length of a stable Flow queue-order key.
pub const MAX_FLOW_QUEUE_ORDER_KEY_BYTES: usize = 128;

/// Immutable opaque secondary ordering key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FlowQueueOrderKey(Box<[u8]>);

impl FlowQueueOrderKey {
    /// Copy a bounded nonempty key. Ordering is unsigned bytewise lexicographic.
    pub fn new(bytes: &[u8]) -> Result<Self, FlowQueueOrderKeyError> {
        if bytes.is_empty() {
            return Err(FlowQueueOrderKeyError::Empty);
        }
        if bytes.len() > MAX_FLOW_QUEUE_ORDER_KEY_BYTES {
            return Err(FlowQueueOrderKeyError::TooLong {
                limit: MAX_FLOW_QUEUE_ORDER_KEY_BYTES,
                actual: bytes.len(),
            });
        }
        Ok(Self(bytes.into()))
    }

    /// Borrow the immutable key bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Invalid stable queue-order key input.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FlowQueueOrderKeyError {
    /// A stable key cannot be empty.
    #[error("queue order key must not be empty")]
    Empty,
    /// The key exceeded the fixed engine bound.
    #[error("queue order key has {actual} bytes; maximum is {limit}")]
    TooLong { limit: usize, actual: usize },
}

/// Immutable per-resource queue-order policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowQueueOrdering {
    /// Preserve native admission-sequence ordering.
    AdmissionOrder,
    /// Order equal-primary waiters by their immutable supplied key.
    StableKey,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SecondaryOrder {
    Admission,
    Stable(FlowQueueOrderKey),
}

/// Derived cache key. Primary priority remains the first ordering component.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct EffectiveQueueKey {
    level: i32,
    secondary: SecondaryOrder,
    enqueue_sequence: u64,
    request: RequestId,
}

impl EffectiveQueueKey {
    pub(crate) fn new(
        level: i32,
        enqueue_sequence: u64,
        request: RequestId,
        ordering: FlowQueueOrdering,
        key: Option<&FlowQueueOrderKey>,
    ) -> Option<Self> {
        let secondary = match (ordering, key) {
            (FlowQueueOrdering::AdmissionOrder, None) => SecondaryOrder::Admission,
            (FlowQueueOrdering::StableKey, Some(key)) => SecondaryOrder::Stable(key.clone()),
            _ => return None,
        };
        Some(Self {
            level,
            secondary,
            enqueue_sequence,
            request,
        })
    }

    pub(crate) fn request(&self) -> RequestId {
        self.request
    }

    pub(crate) fn level(&self) -> i32 {
        self.level
    }

    pub(crate) fn enqueue_sequence(&self) -> u64 {
        self.enqueue_sequence
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ResourceQueueOrderIndexes {
    pub(crate) queue: BTreeSet<EffectiveQueueKey>,
    pub(crate) queue_expected_len: usize,
    pub(crate) preempting: BTreeSet<EffectiveQueueKey>,
    pub(crate) preempting_expected_len: usize,
}

pub(crate) fn queue_ordering_indexes_match(
    resources: &BTreeSet<ResourceId>,
    indexes: &std::collections::BTreeMap<ResourceId, ResourceQueueOrderIndexes>,
) -> bool {
    indexes.len() == resources.len() && indexes.keys().all(|resource| resources.contains(resource))
}

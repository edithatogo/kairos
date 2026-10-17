//! Private derived index of waiting requests allowed to preempt.

use super::PriorityKey;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct PreemptingWaiters {
    pub(super) keys: BTreeSet<PriorityKey>,
    pub(super) expected_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OwnedPreemptingChanges {
    pub(super) old_expected_len: usize,
    pub(super) new_expected_len: usize,
    pub(super) changes: super::queue_delta::QueueChanges<PriorityKey>,
}

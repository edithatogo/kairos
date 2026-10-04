//! Private per-resource index of queued requests with finite deadlines.

use super::{PriorityKey, SimTime};
use std::collections::BTreeSet;

pub(super) type DeadlineKey = (SimTime, PriorityKey);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct WaitingDeadlineIndex {
    pub(super) entries: BTreeSet<DeadlineKey>,
    pub(super) expected_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OwnedDeadlineChanges {
    pub(super) old_expected_len: usize,
    pub(super) new_expected_len: usize,
    pub(super) changes: super::queue_delta::QueueChanges<DeadlineKey>,
}

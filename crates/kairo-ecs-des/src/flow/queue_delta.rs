//! Private copy-on-write overlay for an authoritative ordered queue.
//!
//! The base is borrowed and never changed by staging. Snapshots and exports own
//! only the changed keys; callers drop the overlay before applying its changes.

use std::collections::{btree_set, BTreeSet};
use std::iter::Peekable;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeltaError {
    RemovedKeyMissing,
    AddedKeyAlreadyPresent,
}

#[derive(Clone)]
pub(crate) struct QueueDelta<'a, K: Ord + Clone> {
    base: &'a BTreeSet<K>,
    added: BTreeSet<K>,
    removed: BTreeSet<K>,
}

impl<'a, K: Ord + Clone> QueueDelta<'a, K> {
    pub(crate) fn new(base: &'a BTreeSet<K>) -> Self {
        Self {
            base,
            added: BTreeSet::new(),
            removed: BTreeSet::new(),
        }
    }

    pub(crate) fn contains(&self, key: &K) -> bool {
        if self.removed.contains(key) {
            false
        } else if self.added.contains(key) {
            true
        } else {
            self.base.contains(key)
        }
    }

    /// Inserts a key into the staged view and returns whether it was absent.
    pub(crate) fn insert(&mut self, key: K) -> bool {
        if self.contains(&key) {
            return false;
        }
        if self.base.contains(&key) {
            // The key was removed from the base earlier; restoring it cancels
            // the removal rather than recording a redundant addition.
            self.removed.remove(&key);
        } else {
            self.added.insert(key);
        }
        true
    }

    /// Removes a key from the staged view and returns whether it was present.
    pub(crate) fn remove(&mut self, key: &K) -> bool {
        if !self.contains(key) {
            return false;
        }
        if self.added.remove(key) {
            // An addition followed by removal has no base effect.
            return true;
        }
        debug_assert!(self.base.contains(key));
        self.removed.insert(key.clone());
        true
    }

    pub(crate) fn iter(&self) -> QueueIter<'_, 'a, K> {
        QueueIter {
            base: self.base.iter().peekable(),
            added: self.added.iter().peekable(),
            removed: &self.removed,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.base.len() - self.removed.len() + self.added.len()
    }

    pub(crate) fn export(&self) -> QueueChanges<K> {
        QueueChanges {
            added: self.added.clone(),
            removed: self.removed.clone(),
        }
    }
}

/// A lazy sorted merge of the unchanged base and the overlay's additions.
pub(crate) struct QueueIter<'d, 'a, K: Ord> {
    base: Peekable<btree_set::Iter<'a, K>>,
    added: Peekable<btree_set::Iter<'d, K>>,
    removed: &'d BTreeSet<K>,
}

impl<'d, 'a: 'd, K: Ord> Iterator for QueueIter<'d, 'a, K> {
    type Item = &'d K;

    fn next(&mut self) -> Option<Self::Item> {
        while self
            .base
            .peek()
            .is_some_and(|key| self.removed.contains(*key))
        {
            self.base.next();
        }

        match (self.base.peek(), self.added.peek()) {
            (None, None) => None,
            (Some(_), None) => self.base.next().map(|key| key as &'d K),
            (None, Some(_)) => self.added.next(),
            (Some(base), Some(added)) => match base.cmp(added) {
                std::cmp::Ordering::Less => self.base.next().map(|key| key as &'d K),
                std::cmp::Ordering::Greater => self.added.next(),
                std::cmp::Ordering::Equal => {
                    // The normalized overlay cannot normally contain an
                    // addition already present in base. Advance both to
                    // keep iteration set-like if a caller violates that
                    // invariant through an unstable Ord implementation.
                    self.base.next();
                    self.added.next()
                }
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueueChanges<K: Ord> {
    added: BTreeSet<K>,
    removed: BTreeSet<K>,
}

impl<K: Ord + Clone> QueueChanges<K> {
    pub(crate) fn preflight(&self, base: &BTreeSet<K>) -> Result<(), DeltaError> {
        if self.removed.iter().any(|key| !base.contains(key)) {
            return Err(DeltaError::RemovedKeyMissing);
        }
        if self.added.iter().any(|key| base.contains(key)) {
            return Err(DeltaError::AddedKeyAlreadyPresent);
        }
        Ok(())
    }

    /// Applies after a full preflight, so validation errors never partially
    /// mutate the authoritative set. The base must keep a stable `Ord` relation.
    pub(crate) fn apply(&self, base: &mut BTreeSet<K>) -> Result<(), DeltaError> {
        self.preflight(base)?;
        for key in &self.removed {
            let removed = base.remove(key);
            debug_assert!(removed, "preflighted removal remains present");
        }
        for key in &self.added {
            let inserted = base.insert(key.clone());
            debug_assert!(inserted, "preflighted addition remains absent");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{DeltaError, QueueDelta};
    use std::cell::Cell;
    use std::collections::BTreeSet;
    use std::rc::Rc;

    #[test]
    fn mixed_operations_match_independent_btree_baseline_and_keep_base_unchanged() {
        let base: BTreeSet<i32> = [i32::MIN, -8, -1, 0, 4, 9, i32::MAX].into_iter().collect();
        let mut expected = base.clone();
        let mut delta = QueueDelta::new(&base);
        let operations = [
            (true, i32::MIN),
            (false, i32::MIN),
            (true, i32::MAX),
            (false, i32::MAX),
            (false, 4),
            (true, 4),
            (true, 3),
            (false, 3),
            (true, -9),
            (false, -8),
            (true, -8),
            (false, 9),
            (true, 11),
        ];
        for (insert, key) in operations {
            let expected_changed = if insert {
                expected.insert(key)
            } else {
                expected.remove(&key)
            };
            let delta_changed = if insert {
                delta.insert(key)
            } else {
                delta.remove(&key)
            };
            assert_eq!(
                delta_changed, expected_changed,
                "key={key}, insert={insert}"
            );
            assert_eq!(delta.contains(&key), expected.contains(&key));
            assert_eq!(
                delta.iter().copied().collect::<Vec<_>>(),
                expected.iter().copied().collect::<Vec<_>>()
            );
            assert_eq!(delta.len(), expected.len());
        }
        assert_eq!(
            base,
            [-8, -1, 0, 4, 9, i32::MIN, i32::MAX].into_iter().collect()
        );
    }

    #[test]
    fn equal_key_insert_remove_and_reinsert_cancel_the_delta() {
        let base: BTreeSet<_> = [1, 2, 3].into_iter().collect();
        let mut delta = QueueDelta::new(&base);
        assert!(!delta.insert(2));
        assert!(delta.remove(&2));
        assert!(!delta.contains(&2));
        assert!(delta.insert(2));
        assert!(delta.contains(&2));
        assert!(delta.insert(4));
        assert!(!delta.insert(4));
        assert!(delta.remove(&4));
        assert!(!delta.contains(&4));
        assert_eq!(delta.len(), base.len());
        let changes = delta.export();
        assert!(changes.added.is_empty());
        assert!(changes.removed.is_empty());
        assert_eq!(delta.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(base, [1, 2, 3].into_iter().collect());
    }

    #[test]
    fn ordered_iterator_and_cloned_snapshot_are_independent() {
        let base: BTreeSet<_> = [1, 4, 7, 10].into_iter().collect();
        let mut original = QueueDelta::new(&base);
        original.remove(&4);
        original.insert(5);
        original.insert(12);
        let mut original_view = base.clone();
        original_view.remove(&4);
        original_view.insert(5);
        original_view.insert(12);

        let mut snapshot = original.clone();
        original.remove(&1);
        original.insert(3);
        snapshot.remove(&10);
        snapshot.insert(8);

        let mut expected_original = original_view.clone();
        expected_original.remove(&1);
        expected_original.insert(3);
        let mut expected_snapshot = original_view;
        expected_snapshot.remove(&10);
        expected_snapshot.insert(8);
        assert_eq!(
            original.iter().copied().collect::<Vec<_>>(),
            expected_original.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(
            snapshot.iter().copied().collect::<Vec<_>>(),
            expected_snapshot.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(base, [1, 4, 7, 10].into_iter().collect());
    }

    #[test]
    fn export_preflight_and_apply_match_independent_baseline() {
        let base: BTreeSet<_> = [-4, 0, 1, 7, 11].into_iter().collect();
        let mut expected = base.clone();
        let mut delta = QueueDelta::new(&base);
        for key in [0, 7] {
            assert!(delta.remove(&key));
            assert!(expected.remove(&key));
        }
        for key in [7, 23, i32::MIN, 3] {
            assert_eq!(delta.insert(key), expected.insert(key));
        }
        let changes = delta.export();
        assert_eq!(changes.preflight(&base), Ok(()));
        let mut applied = base.clone();
        changes.apply(&mut applied).unwrap();
        assert_eq!(applied, expected);
        assert_eq!(base, [-4, 0, 1, 7, 11].into_iter().collect());
    }

    #[test]
    fn tuple_key_rekey_preserves_tie_order_and_applies_export() {
        type QueueKey = (u8, u64, u64);
        let base: BTreeSet<QueueKey> = [(2, 4, 10), (2, 4, 11), (3, 1, 12), (3, 9, 13)]
            .into_iter()
            .collect();
        let original = base.clone();
        let mut expected = base.clone();
        let mut delta = QueueDelta::new(&base);

        let old_key = (2, 4, 10);
        let reprioritized_key = (3, 4, 10);
        assert!(delta.remove(&old_key));
        assert!(expected.remove(&old_key));
        assert!(delta.insert(reprioritized_key));
        assert!(expected.insert(reprioritized_key));
        assert_eq!(
            delta.iter().copied().collect::<Vec<_>>(),
            expected.iter().copied().collect::<Vec<_>>()
        );

        let changes = delta.export();
        assert_eq!(changes.preflight(&base), Ok(()));
        let mut applied = base.clone();
        changes.apply(&mut applied).unwrap();
        assert_eq!(applied, expected);
        assert_eq!(base, original);
    }

    #[test]
    fn failed_apply_is_atomic_for_missing_remove_and_present_addition() {
        let base: BTreeSet<_> = [1, 2, 3].into_iter().collect();
        let mut delta = QueueDelta::new(&base);
        assert!(delta.remove(&1));
        assert!(delta.insert(4));
        let changes = delta.export();

        let mut missing_remove: BTreeSet<_> = [2, 3].into_iter().collect();
        let before = missing_remove.clone();
        assert_eq!(
            changes.apply(&mut missing_remove),
            Err(DeltaError::RemovedKeyMissing)
        );
        assert_eq!(missing_remove, before);

        let mut present_addition: BTreeSet<_> = [1, 2, 3, 4].into_iter().collect();
        let before = present_addition.clone();
        assert_eq!(
            changes.preflight(&present_addition),
            Err(DeltaError::AddedKeyAlreadyPresent)
        );
        assert_eq!(
            changes.apply(&mut present_addition),
            Err(DeltaError::AddedKeyAlreadyPresent)
        );
        assert_eq!(present_addition, before);
    }

    #[derive(Debug)]
    struct CloneProbe {
        value: i32,
        clones: Rc<Cell<usize>>,
    }

    impl PartialEq for CloneProbe {
        fn eq(&self, other: &Self) -> bool {
            self.value == other.value
        }
    }

    impl Eq for CloneProbe {}

    impl Clone for CloneProbe {
        fn clone(&self) -> Self {
            self.clones.set(self.clones.get() + 1);
            Self {
                value: self.value,
                clones: Rc::clone(&self.clones),
            }
        }
    }

    impl PartialOrd for CloneProbe {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for CloneProbe {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.value.cmp(&other.value)
        }
    }

    // Interior mutability observes Clone calls only; ordering is by immutable value.
    #[allow(clippy::mutable_key_type)]
    #[test]
    fn snapshot_clone_cost_scales_with_delta_not_large_base() {
        let clones = Rc::new(Cell::new(0));
        let base: BTreeSet<_> = (0..20_000)
            .map(|value| CloneProbe {
                value,
                clones: Rc::clone(&clones),
            })
            .collect();
        let mut delta = QueueDelta::new(&base);
        delta.remove(
            base.get(&CloneProbe {
                value: 10,
                clones: Rc::clone(&clones),
            })
            .unwrap(),
        );
        delta.remove(
            base.get(&CloneProbe {
                value: 19_990,
                clones: Rc::clone(&clones),
            })
            .unwrap(),
        );
        delta.insert(CloneProbe {
            value: 20_001,
            clones: Rc::clone(&clones),
        });
        delta.insert(CloneProbe {
            value: 20_002,
            clones: Rc::clone(&clones),
        });
        assert_eq!(delta.len(), base.len());

        clones.set(0);
        let snapshot = delta.clone();
        assert_eq!(clones.get(), 4);
        assert_eq!(snapshot.len(), delta.len());
        assert_eq!(base.len(), 20_000);
    }
}

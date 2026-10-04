//! Private copy-on-write overlay for retained Flow records.
//!
//! The authoritative record table remains owned by its caller. A delta borrows
//! only a lookup function and stores records changed by this boundary.

use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct RecordDelta<'a, K: Ord + Clone, V: Clone> {
    lookup: &'a dyn Fn(&K) -> Option<&'a V>,
    writes: BTreeMap<K, V>,
}

impl<'a, K: Ord + Clone, V: Clone> RecordDelta<'a, K, V> {
    pub(crate) fn new(lookup: &'a dyn Fn(&K) -> Option<&'a V>) -> Self {
        Self {
            lookup,
            writes: BTreeMap::new(),
        }
    }

    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        self.writes.get(key).or_else(|| (self.lookup)(key))
    }

    pub(crate) fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if !self.writes.contains_key(key) {
            let value = (self.lookup)(key)?.clone();
            self.writes.insert(key.clone(), value);
        }
        self.writes.get_mut(key)
    }

    #[cfg(test)]
    pub(crate) fn insert(&mut self, key: K, value: V) {
        self.writes.insert(key, value);
    }

    #[cfg(test)]
    pub(crate) fn changed_ids(&self) -> impl Iterator<Item = &K> {
        self.writes.keys()
    }

    pub(crate) fn into_writes(self) -> BTreeMap<K, V> {
        self.writes
    }
}

#[cfg(test)]
mod tests {
    use super::RecordDelta;
    use std::cell::Cell;
    use std::collections::{BTreeMap, BTreeSet};
    use std::rc::Rc;

    #[derive(Debug, PartialEq, Eq)]
    struct Counted {
        value: i32,
        clones: Rc<Cell<usize>>,
    }

    impl Clone for Counted {
        fn clone(&self) -> Self {
            self.clones.set(self.clones.get() + 1);
            Self {
                value: self.value,
                clones: Rc::clone(&self.clones),
            }
        }
    }

    fn counted(value: i32, clones: &Rc<Cell<usize>>) -> Counted {
        Counted {
            value,
            clones: Rc::clone(clones),
        }
    }

    #[test]
    fn reads_through_and_writes_are_boundary_local() {
        let base = BTreeMap::from([(1, 10), (3, 30)]);
        let lookup = |key: &i32| base.get(key);
        let mut delta = RecordDelta::new(&lookup);

        assert_eq!(delta.get(&1), Some(&10));
        assert_eq!(delta.get(&2), None);
        assert_eq!(delta.get_mut(&2), None);
        assert_eq!(delta.get_mut(&1), Some(&mut 10));
        *delta.get_mut(&1).expect("base value") += 2;
        delta.insert(2, 20);

        assert_eq!(delta.get(&1), Some(&12));
        assert_eq!(delta.get(&2), Some(&20));
        assert_eq!(delta.get(&3), Some(&30));
        assert_eq!(base, BTreeMap::from([(1, 10), (3, 30)]));
        assert_eq!(delta.changed_ids().copied().collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn mutable_lookup_clones_only_once_and_reads_clone_nothing() {
        let clones = Rc::new(Cell::new(0));
        let base = BTreeMap::from([(7, counted(70, &clones))]);
        let lookup = |key: &i32| base.get(key);
        let mut delta = RecordDelta::new(&lookup);

        assert_eq!(delta.get(&7).map(|value| value.value), Some(70));
        assert_eq!(clones.get(), 0);
        delta.get_mut(&7).expect("base value").value += 1;
        assert_eq!(clones.get(), 1);
        delta.get_mut(&7).expect("delta value").value += 1;
        assert_eq!(clones.get(), 1);
        assert_eq!(delta.get(&7).map(|value| value.value), Some(72));
        assert_eq!(clones.get(), 1);
        assert_eq!(base[&7].value, 70);
    }

    #[test]
    fn insert_replaces_an_existing_base_key_without_mutating_the_base() {
        let base = BTreeMap::from([(5, 50), (8, 80)]);
        let lookup = |key: &i32| base.get(key);
        let mut delta = RecordDelta::new(&lookup);

        delta.insert(5, 55);
        assert_eq!(delta.get(&5), Some(&55));
        assert_eq!(delta.changed_ids().copied().collect::<Vec<_>>(), [5]);
        assert_eq!(base, BTreeMap::from([(5, 50), (8, 80)]));

        let writes = delta.into_writes();
        assert_eq!(writes, BTreeMap::from([(5, 55)]));
    }

    #[test]
    fn clone_is_an_independent_snapshot_and_export_owns_exact_writes() {
        let clones = Rc::new(Cell::new(0));
        let base = BTreeMap::from([(1, counted(10, &clones)), (2, counted(20, &clones))]);
        let lookup = |key: &i32| base.get(key);
        let mut boundary = RecordDelta::new(&lookup);
        boundary.get_mut(&1).expect("base value").value = 11;
        boundary.insert(4, counted(40, &clones));

        let mut snapshot = boundary.clone();
        assert_eq!(
            clones.get(),
            3,
            "one prior lazy clone plus two changed writes"
        );
        snapshot.get_mut(&1).expect("snapshot write").value = 99;
        snapshot.insert(3, counted(30, &clones));
        assert_eq!(boundary.get(&1).map(|value| value.value), Some(11));
        assert_eq!(boundary.get(&3), None);
        assert_eq!(base[&1].value, 10);
        assert_eq!(base[&2].value, 20);

        let writes = boundary.into_writes();
        assert_eq!(
            writes.keys().copied().collect::<BTreeSet<_>>(),
            [1, 4].into()
        );
        assert_eq!(writes[&1].value, 11);
        assert_eq!(writes[&4].value, 40);
        assert!(
            !writes.contains_key(&2),
            "unchanged base values are not exported"
        );
    }

    #[test]
    fn readonly_registry_snapshots_clone_no_records() {
        let clones = Rc::new(Cell::new(0));
        let base = (0..10_000)
            .map(|key| (key, counted(key, &clones)))
            .collect::<BTreeMap<_, _>>();
        let lookup = |key: &i32| base.get(key);
        let delta = RecordDelta::new(&lookup);

        for key in [0, 5_000, 9_999] {
            assert_eq!(delta.get(&key).map(|value| value.value), Some(key));
        }
        let snapshot = delta.clone();
        assert_eq!(snapshot.changed_ids().count(), 0);
        assert_eq!(clones.get(), 0);
        assert_eq!(base.len(), 10_000);
    }
}

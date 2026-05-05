#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TwinStateSnapshot {
    pub tick: u64,
    pub checksum: u64,
    pub entries: Vec<TwinStateEntry>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TwinStateEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TwinStateDiff {
    pub from_tick: u64,
    pub to_tick: u64,
    pub changed: Vec<TwinStateEntry>,
    pub removed: Vec<String>,
}

impl TwinStateSnapshot {
    pub fn new(tick: u64, mut entries: Vec<TwinStateEntry>) -> Self {
        entries.sort_by(|left, right| left.key.cmp(&right.key));
        let checksum = checksum(tick, &entries);
        Self {
            tick,
            checksum,
            entries,
        }
    }

    pub fn diff(&self, next: &Self) -> TwinStateDiff {
        let changed = next
            .entries
            .iter()
            .filter(|entry| {
                self.entries
                    .iter()
                    .find(|candidate| candidate.key == entry.key)
                    .map(|candidate| candidate.value != entry.value)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        let removed = self
            .entries
            .iter()
            .filter(|entry| {
                !next
                    .entries
                    .iter()
                    .any(|candidate| candidate.key == entry.key)
            })
            .map(|entry| entry.key.clone())
            .collect();

        TwinStateDiff {
            from_tick: self.tick,
            to_tick: next.tick,
            changed,
            removed,
        }
    }

    pub fn apply(&self, diff: &TwinStateDiff) -> Self {
        let mut entries: Vec<TwinStateEntry> = self
            .entries
            .iter()
            .filter(|entry| !diff.removed.iter().any(|removed| removed == &entry.key))
            .cloned()
            .collect();

        for changed in &diff.changed {
            if let Some(existing) = entries.iter_mut().find(|entry| entry.key == changed.key) {
                existing.value = changed.value.clone();
            } else {
                entries.push(changed.clone());
            }
        }

        Self::new(diff.to_tick, entries)
    }
}

impl TwinStateEntry {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

fn checksum(tick: u64, entries: &[TwinStateEntry]) -> u64 {
    let mut hash = tick.wrapping_mul(1_099_511_628_211);
    for entry in entries {
        for byte in entry.key.bytes().chain(entry.value.bytes()) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_diff_apply_round_trips() {
        let before = TwinStateSnapshot::new(
            1,
            vec![
                TwinStateEntry::new("position", "1.0"),
                TwinStateEntry::new("velocity", "2.0"),
            ],
        );
        let after = TwinStateSnapshot::new(
            2,
            vec![
                TwinStateEntry::new("position", "3.0"),
                TwinStateEntry::new("temperature", "20.0"),
            ],
        );

        let diff = before.diff(&after);
        assert_eq!(before.apply(&diff), after);
    }
}

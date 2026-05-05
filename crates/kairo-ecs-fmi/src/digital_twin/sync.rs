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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TwinStateError {
    message: String,
}

impl TwinStateError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TwinStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TwinStateError {}

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

    pub fn try_new(tick: u64, entries: Vec<TwinStateEntry>) -> Result<Self, TwinStateError> {
        validate_entries("snapshot", &entries)?;
        Ok(Self::new(tick, entries))
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

    pub fn try_apply(&self, diff: &TwinStateDiff) -> Result<Self, TwinStateError> {
        if diff.from_tick != self.tick {
            return Err(TwinStateError::new(format!(
                "diff from_tick {} does not match snapshot tick {}",
                diff.from_tick, self.tick
            )));
        }
        validate_entries("changed entries", &diff.changed)?;
        if diff.removed.iter().any(|key| key.trim().is_empty()) {
            return Err(TwinStateError::new("removed keys must not be empty"));
        }
        Ok(self.apply(diff))
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

fn validate_entries(label: &str, entries: &[TwinStateEntry]) -> Result<(), TwinStateError> {
    let mut keys = std::collections::BTreeSet::new();
    for entry in entries {
        if entry.key.trim().is_empty() {
            return Err(TwinStateError::new(format!(
                "{label} keys must not be empty"
            )));
        }
        if !keys.insert(entry.key.clone()) {
            return Err(TwinStateError::new(format!(
                "{label} contains duplicate key {}",
                entry.key
            )));
        }
    }
    Ok(())
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
        assert_eq!(before.try_apply(&diff).expect("checked apply"), after);
    }

    #[test]
    fn checked_snapshot_rejects_duplicate_keys() {
        let error = TwinStateSnapshot::try_new(
            1,
            vec![
                TwinStateEntry::new("position", "1.0"),
                TwinStateEntry::new("position", "2.0"),
            ],
        )
        .expect_err("duplicate key should fail");

        assert_eq!(
            error.to_string(),
            "snapshot contains duplicate key position"
        );
    }

    #[test]
    fn checked_apply_rejects_wrong_base_tick() {
        let before = TwinStateSnapshot::try_new(1, vec![TwinStateEntry::new("position", "1.0")])
            .expect("snapshot");
        let diff = TwinStateDiff {
            from_tick: 0,
            to_tick: 2,
            changed: vec![TwinStateEntry::new("position", "2.0")],
            removed: Vec::new(),
        };

        let error = before.try_apply(&diff).expect_err("wrong base should fail");

        assert_eq!(
            error.to_string(),
            "diff from_tick 0 does not match snapshot tick 1"
        );
    }
}

//! Fixed-capacity logical component bitsets with rollback-safe handles.
//!
//! Handles carry a process-unique instance identity and a coarse global epoch.
//! Every successful insertion, removal, or restore advances that epoch, so all
//! previously issued handles become invalid, including handles for unaffected
//! slots. Snapshots contain only capacity and logical bits; they never preserve
//! handle authority or instance identity.

use std::sync::atomic::{AtomicU64, Ordering};

/// Maximum number of slots accepted by a [`GenerationBitset`].
pub const MAX_GENERATION_SLOTS: u32 = 65_536;

static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

/// A typed failure from a generation bitset operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationBitsetError {
    /// The requested slot is outside the fixed capacity.
    SlotOutOfRange { slot: u32, slot_count: u32 },
    /// Insertion was requested for a slot that is already active.
    AlreadyActive { slot: u32 },
    /// A handle or lookup refers to a slot that is not active.
    InactiveSlot { slot: u32 },
    /// A handle belongs to another instance or an earlier epoch.
    StaleHandle,
    /// A snapshot has a different capacity, malformed storage, or invalid tail bits.
    SnapshotShapeMismatch,
    /// The process-unique instance identity counter is exhausted.
    InstanceExhausted,
    /// The non-rollback epoch cannot be advanced without wrapping.
    EpochExhausted,
    /// A required vector allocation failed.
    AllocationFailed,
    /// The requested capacity exceeds [`MAX_GENERATION_SLOTS`].
    CapacityExceeded { requested: u32, maximum: u32 },
}

/// Opaque authority to validate one active slot in one bitset epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationHandle {
    instance_id: u64,
    epoch: u64,
    slot: u32,
}

/// A cloneable logical snapshot. It deliberately excludes handle authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationBitsetSnapshot {
    slot_count: u32,
    bits: Vec<u64>,
}

/// A fixed-capacity set of active slots with conservative stale-handle checks.
#[derive(Debug)]
pub struct GenerationBitset {
    instance_id: u64,
    epoch: u64,
    slot_count: u32,
    bits: Vec<u64>,
}

impl GenerationBitset {
    /// Creates an empty bitset with the requested fixed slot capacity.
    pub fn new(slot_count: u32) -> Result<Self, GenerationBitsetError> {
        if slot_count > MAX_GENERATION_SLOTS {
            return Err(GenerationBitsetError::CapacityExceeded {
                requested: slot_count,
                maximum: MAX_GENERATION_SLOTS,
            });
        }

        let word_count = words_for(slot_count);
        let mut bits = Vec::new();
        bits.try_reserve_exact(word_count)
            .map_err(|_| GenerationBitsetError::AllocationFailed)?;
        bits.resize(word_count, 0);

        let instance_id = allocate_instance_id(&NEXT_INSTANCE_ID)?;
        Ok(Self {
            instance_id,
            epoch: 0,
            slot_count,
            bits,
        })
    }

    /// Returns the fixed capacity in slots.
    pub fn slot_count(&self) -> u32 {
        self.slot_count
    }

    /// Returns whether a slot is active.
    pub fn is_active(&self, slot: u32) -> Result<bool, GenerationBitsetError> {
        self.check_slot(slot)?;
        Ok(self.bit_is_set(slot))
    }

    /// Activates an inactive slot and returns a handle for the new epoch.
    pub fn insert(&mut self, slot: u32) -> Result<GenerationHandle, GenerationBitsetError> {
        self.check_slot(slot)?;
        if self.bit_is_set(slot) {
            return Err(GenerationBitsetError::AlreadyActive { slot });
        }
        let next_epoch = self.next_epoch()?;
        self.set_bit(slot, true);
        self.epoch = next_epoch;
        Ok(self.make_handle(slot))
    }

    /// Returns a current handle for an active slot without changing the epoch.
    pub fn handle(&self, slot: u32) -> Result<GenerationHandle, GenerationBitsetError> {
        self.check_slot(slot)?;
        if !self.bit_is_set(slot) {
            return Err(GenerationBitsetError::InactiveSlot { slot });
        }
        Ok(self.make_handle(slot))
    }

    /// Validates a handle and returns its active slot.
    pub fn validate(&self, handle: GenerationHandle) -> Result<u32, GenerationBitsetError> {
        if handle.instance_id != self.instance_id || handle.epoch != self.epoch {
            return Err(GenerationBitsetError::StaleHandle);
        }
        self.check_slot(handle.slot)?;
        if !self.bit_is_set(handle.slot) {
            return Err(GenerationBitsetError::InactiveSlot { slot: handle.slot });
        }
        Ok(handle.slot)
    }

    /// Removes the active slot referenced by a current handle.
    pub fn remove(&mut self, handle: GenerationHandle) -> Result<(), GenerationBitsetError> {
        let slot = self.validate(handle)?;
        let next_epoch = self.next_epoch()?;
        self.set_bit(slot, false);
        self.epoch = next_epoch;
        Ok(())
    }

    /// Captures capacity and logical active bits, without handle authority.
    pub fn snapshot(&self) -> GenerationBitsetSnapshot {
        GenerationBitsetSnapshot {
            slot_count: self.slot_count,
            bits: self.bits.clone(),
        }
    }

    /// Restores logical bits while preserving this instance's identity.
    ///
    /// A successful restore always advances the epoch and invalidates every
    /// existing handle, even when the snapshot has identical logical contents.
    pub fn restore(
        &mut self,
        snapshot: &GenerationBitsetSnapshot,
    ) -> Result<(), GenerationBitsetError> {
        if snapshot.slot_count != self.slot_count
            || snapshot.bits.len() != words_for(snapshot.slot_count)
            || !tail_bits_are_clear(snapshot.slot_count, &snapshot.bits)
        {
            return Err(GenerationBitsetError::SnapshotShapeMismatch);
        }

        let next_epoch = self.next_epoch()?;
        let mut restored_bits = Vec::new();
        restored_bits
            .try_reserve_exact(snapshot.bits.len())
            .map_err(|_| GenerationBitsetError::AllocationFailed)?;
        restored_bits.extend_from_slice(&snapshot.bits);

        self.bits = restored_bits;
        self.epoch = next_epoch;
        Ok(())
    }

    fn check_slot(&self, slot: u32) -> Result<(), GenerationBitsetError> {
        if slot >= self.slot_count {
            return Err(GenerationBitsetError::SlotOutOfRange {
                slot,
                slot_count: self.slot_count,
            });
        }
        Ok(())
    }

    fn bit_is_set(&self, slot: u32) -> bool {
        let word = (slot / 64) as usize;
        let mask = 1u64 << (slot % 64);
        self.bits[word] & mask != 0
    }

    fn set_bit(&mut self, slot: u32, active: bool) {
        let word = (slot / 64) as usize;
        let mask = 1u64 << (slot % 64);
        if active {
            self.bits[word] |= mask;
        } else {
            self.bits[word] &= !mask;
        }
    }

    fn next_epoch(&self) -> Result<u64, GenerationBitsetError> {
        self.epoch
            .checked_add(1)
            .ok_or(GenerationBitsetError::EpochExhausted)
    }

    fn make_handle(&self, slot: u32) -> GenerationHandle {
        GenerationHandle {
            instance_id: self.instance_id,
            epoch: self.epoch,
            slot,
        }
    }
}

fn words_for(slot_count: u32) -> usize {
    (slot_count as usize).div_ceil(64)
}

fn tail_bits_are_clear(slot_count: u32, bits: &[u64]) -> bool {
    let remainder = slot_count % 64;
    if remainder == 0 || bits.is_empty() {
        return true;
    }
    let valid_mask = (1u64 << remainder) - 1;
    bits.last().is_some_and(|last| last & !valid_mask == 0)
}

fn allocate_instance_id(counter: &AtomicU64) -> Result<u64, GenerationBitsetError> {
    let mut current = counter.load(Ordering::Acquire);
    loop {
        let next = current
            .checked_add(1)
            .ok_or(GenerationBitsetError::InstanceExhausted)?;
        match counter.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return Ok(current),
            Err(observed) => current = observed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_word_edges_last_slot_and_first_invalid_slot() {
        let mut bitset = GenerationBitset::new(130).unwrap();
        for slot in [0, 63, 64, 129] {
            let handle = bitset.insert(slot).unwrap();
            assert_eq!(bitset.validate(handle), Ok(slot));
            assert!(bitset.is_active(slot).unwrap());
        }
        assert_eq!(
            bitset.is_active(130),
            Err(GenerationBitsetError::SlotOutOfRange {
                slot: 130,
                slot_count: 130
            })
        );
        assert_eq!(bitset.snapshot().bits, vec![1 | (1 << 63), 1, 1 << 1]);
    }

    #[test]
    fn accepts_zero_and_max_capacity_and_rejects_oversized_capacity() {
        let empty = GenerationBitset::new(0).unwrap();
        assert_eq!(empty.slot_count(), 0);
        assert_eq!(empty.snapshot().bits, Vec::<u64>::new());
        assert!(matches!(
            empty.is_active(0),
            Err(GenerationBitsetError::SlotOutOfRange { .. })
        ));
        let maximum = GenerationBitset::new(MAX_GENERATION_SLOTS).unwrap();
        assert_eq!(maximum.snapshot().bits.len(), 1024);
        assert!(matches!(
            GenerationBitset::new(MAX_GENERATION_SLOTS + 1),
            Err(GenerationBitsetError::CapacityExceeded { requested, maximum })
                if requested == MAX_GENERATION_SLOTS + 1
                    && maximum == MAX_GENERATION_SLOTS
        ));
    }

    #[test]
    fn duplicate_insert_is_rejected_without_changing_state_or_epoch() {
        let mut bitset = GenerationBitset::new(2).unwrap();
        let handle = bitset.insert(0).unwrap();
        let snapshot = bitset.snapshot();
        let epoch = bitset.epoch;
        assert_eq!(
            bitset.insert(0),
            Err(GenerationBitsetError::AlreadyActive { slot: 0 })
        );
        assert_eq!(bitset.snapshot(), snapshot);
        assert_eq!(bitset.epoch, epoch);
        assert_eq!(bitset.validate(handle), Ok(0));
    }

    #[test]
    fn every_successful_insert_invalidates_prior_handles() {
        let mut bitset = GenerationBitset::new(4).unwrap();
        let first = bitset.insert(0).unwrap();
        let second = bitset.insert(1).unwrap();
        assert_eq!(
            bitset.validate(first),
            Err(GenerationBitsetError::StaleHandle)
        );
        assert_eq!(bitset.validate(second), Ok(1));
        assert_eq!(bitset.handle(0).unwrap().epoch, bitset.epoch);
    }

    #[test]
    fn removal_and_reinsertion_never_revive_old_handles() {
        let mut bitset = GenerationBitset::new(2).unwrap();
        let original = bitset.insert(0).unwrap();
        bitset.remove(original).unwrap();
        assert_eq!(
            bitset.validate(original),
            Err(GenerationBitsetError::StaleHandle)
        );
        let reinserted = bitset.insert(0).unwrap();
        assert_eq!(
            bitset.validate(original),
            Err(GenerationBitsetError::StaleHandle)
        );
        assert_eq!(bitset.validate(reinserted), Ok(0));
    }

    #[test]
    fn rejects_inactive_slot_and_foreign_instance_handles() {
        let mut left = GenerationBitset::new(2).unwrap();
        let mut right = GenerationBitset::new(2).unwrap();
        assert_eq!(
            left.handle(0),
            Err(GenerationBitsetError::InactiveSlot { slot: 0 })
        );
        let foreign = right.insert(0).unwrap();
        assert_eq!(
            left.validate(foreign),
            Err(GenerationBitsetError::StaleHandle)
        );
        let current = left.insert(0).unwrap();
        assert_eq!(
            right.remove(current),
            Err(GenerationBitsetError::StaleHandle)
        );
    }

    #[test]
    fn restore_invalidates_all_handles_and_keeps_destination_identity() {
        let mut bitset = GenerationBitset::new(3).unwrap();
        let prior = bitset.insert(0).unwrap();
        let destination_identity = bitset.instance_id;
        let snapshot = bitset.snapshot();
        bitset.insert(1).unwrap();
        let before_restore = bitset.handle(1).unwrap();
        bitset.restore(&snapshot).unwrap();
        assert_eq!(bitset.instance_id, destination_identity);
        assert_eq!(
            bitset.validate(prior),
            Err(GenerationBitsetError::StaleHandle)
        );
        assert_eq!(
            bitset.validate(before_restore),
            Err(GenerationBitsetError::StaleHandle)
        );
        assert!(bitset.is_active(0).unwrap());
        assert!(!bitset.is_active(1).unwrap());
        let fresh = bitset.handle(0).unwrap();
        assert_eq!(bitset.validate(fresh), Ok(0));
    }

    #[test]
    fn identical_foreign_snapshot_restores_bits_without_transferring_authority() {
        let mut source = GenerationBitset::new(2).unwrap();
        source.insert(1).unwrap();
        let snapshot = source.snapshot();
        let mut destination = GenerationBitset::new(2).unwrap();
        let before = destination.insert(0).unwrap();
        destination.restore(&snapshot).unwrap();
        assert_eq!(
            destination.validate(before),
            Err(GenerationBitsetError::StaleHandle)
        );
        assert!(!destination.is_active(0).unwrap());
        assert!(destination.is_active(1).unwrap());
        assert_eq!(destination.validate(destination.handle(1).unwrap()), Ok(1));
        assert_eq!(source.validate(source.handle(1).unwrap()), Ok(1));
    }

    #[test]
    fn malformed_or_mismatched_snapshots_leave_state_unchanged() {
        let mut bitset = GenerationBitset::new(65).unwrap();
        let handle = bitset.insert(64).unwrap();
        let before = bitset.snapshot();
        let epoch = bitset.epoch;

        let mut wrong_capacity = before.clone();
        wrong_capacity.slot_count = 64;
        assert_eq!(
            bitset.restore(&wrong_capacity),
            Err(GenerationBitsetError::SnapshotShapeMismatch)
        );

        let mut wrong_length = before.clone();
        wrong_length.bits.pop();
        assert_eq!(
            bitset.restore(&wrong_length),
            Err(GenerationBitsetError::SnapshotShapeMismatch)
        );

        let mut invalid_tail = before.clone();
        invalid_tail.bits[1] |= 1 << 1;
        assert_eq!(
            bitset.restore(&invalid_tail),
            Err(GenerationBitsetError::SnapshotShapeMismatch)
        );
        assert_eq!(bitset.snapshot(), before);
        assert_eq!(bitset.epoch, epoch);
        assert_eq!(bitset.validate(handle), Ok(64));
    }

    #[test]
    fn epoch_exhaustion_leaves_insert_remove_and_restore_unchanged() {
        let mut bitset = GenerationBitset::new(2).unwrap();
        bitset.insert(0).unwrap();
        bitset.epoch = u64::MAX;
        let before = bitset.snapshot();
        let current_handle = bitset.handle(0).unwrap();
        assert_eq!(bitset.insert(1), Err(GenerationBitsetError::EpochExhausted));
        assert_eq!(
            bitset.remove(current_handle),
            Err(GenerationBitsetError::EpochExhausted)
        );
        let restore_source = GenerationBitset::new(2).unwrap().snapshot();
        assert_eq!(
            bitset.restore(&restore_source),
            Err(GenerationBitsetError::EpochExhausted)
        );
        assert_eq!(bitset.snapshot(), before);
        assert_eq!(bitset.epoch, u64::MAX);
        assert_eq!(bitset.validate(current_handle), Ok(0));
    }

    #[test]
    fn instance_allocator_rejects_exhaustion_without_wrapping() {
        let counter = AtomicU64::new(u64::MAX);
        assert_eq!(
            allocate_instance_id(&counter),
            Err(GenerationBitsetError::InstanceExhausted)
        );
        assert_eq!(counter.load(Ordering::Acquire), u64::MAX);
    }
}

//! Experimental owned checkpoint DTOs for the entity allocator.
//!
//! These engine-native values are not a byte format and do not include
//! component storage or registry state.

use crate::{EntitySlot, World};
use kairo_ecs_types::EntityId;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt::{Display, Formatter};

const WORLD_CHECKPOINT_VERSION_V1: u32 = 1;

/// Limits allocations and cloning performed by world checkpoint operations.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldCheckpointLimits {
    pub max_slots: usize,
}

/// One entity-index slot in a versioned world checkpoint.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldSlotCheckpointV1 {
    pub generation: u32,
    pub alive: bool,
}

/// Complete allocator state required to preserve entity allocation behavior.
///
/// `free_indices` retains the allocator's LIFO stack order, while
/// `live_entities` retains the dense iteration order. This is distinct from
/// the sorted telemetry `WorldSnapshot`.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldCheckpointV1 {
    pub version: u32,
    pub slots: Vec<WorldSlotCheckpointV1>,
    pub free_indices: Vec<u64>,
    pub live_entities: Vec<EntityId>,
}

/// Errors produced while capturing or validating allocator checkpoint state.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldCheckpointError {
    UnsupportedVersion(u32),
    LimitExceeded,
    InvalidState,
    IndexOverflow,
    AllocationFailed,
}

impl Display for WorldCheckpointError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported world checkpoint version: {version}")
            }
            Self::LimitExceeded => f.write_str("world checkpoint exceeds the configured limit"),
            Self::InvalidState => f.write_str("world checkpoint allocator state is inconsistent"),
            Self::IndexOverflow => f.write_str("world checkpoint index exceeds platform limits"),
            Self::AllocationFailed => f.write_str("world checkpoint allocation failed"),
        }
    }
}

impl Error for WorldCheckpointError {}

impl World {
    /// Export the complete entity allocator state without changing this world.
    #[doc(hidden)]
    pub fn checkpoint_state(
        &self,
        limits: WorldCheckpointLimits,
    ) -> Result<WorldCheckpointV1, WorldCheckpointError> {
        if self.slots.len() > limits.max_slots
            || self.free_indices.len() > limits.max_slots
            || self.live_entities.len() > limits.max_slots
        {
            return Err(WorldCheckpointError::LimitExceeded);
        }

        let mut slots = Vec::new();
        slots
            .try_reserve_exact(self.slots.len())
            .map_err(|_| WorldCheckpointError::AllocationFailed)?;
        for slot in &self.slots {
            slots.push(WorldSlotCheckpointV1 {
                generation: slot.generation,
                alive: slot.alive,
            });
        }
        let mut free_indices = Vec::new();
        free_indices
            .try_reserve_exact(self.free_indices.len())
            .map_err(|_| WorldCheckpointError::AllocationFailed)?;
        free_indices.extend(self.free_indices.iter().copied());
        let mut live_entities = Vec::new();
        live_entities
            .try_reserve_exact(self.live_entities.len())
            .map_err(|_| WorldCheckpointError::AllocationFailed)?;
        live_entities.extend(self.live_entities.iter().copied());
        Ok(WorldCheckpointV1 {
            version: WORLD_CHECKPOINT_VERSION_V1,
            slots,
            free_indices,
            live_entities,
        })
    }

    /// Validate and reconstruct a world from an owned allocator checkpoint.
    ///
    /// No reconstructed world storage is allocated until every reference,
    /// generation, and live/free partition has passed validation.
    #[doc(hidden)]
    pub fn from_checkpoint_state(
        state: WorldCheckpointV1,
        limits: WorldCheckpointLimits,
    ) -> Result<Self, WorldCheckpointError> {
        if state.version != WORLD_CHECKPOINT_VERSION_V1 {
            return Err(WorldCheckpointError::UnsupportedVersion(state.version));
        }

        let slot_count = state.slots.len();
        if slot_count > limits.max_slots
            || state.free_indices.len() > limits.max_slots
            || state.live_entities.len() > limits.max_slots
        {
            return Err(WorldCheckpointError::LimitExceeded);
        }
        if state
            .free_indices
            .len()
            .checked_add(state.live_entities.len())
            != Some(slot_count)
        {
            return Err(WorldCheckpointError::InvalidState);
        }

        // All input lengths are capped before this bounded one-byte scratch
        // allocation. It supports linear validation while no destination World
        // storage is constructed until every invariant has passed.
        let mut membership = Vec::new();
        membership
            .try_reserve_exact(slot_count)
            .map_err(|_| WorldCheckpointError::AllocationFailed)?;
        membership.resize(slot_count, 0_u8);

        for entity in &state.live_entities {
            let index =
                usize::try_from(entity.index).map_err(|_| WorldCheckpointError::IndexOverflow)?;
            let Some(slot) = state.slots.get(index) else {
                return Err(WorldCheckpointError::InvalidState);
            };
            if !slot.alive || slot.generation != entity.generation || membership[index] != 0 {
                return Err(WorldCheckpointError::InvalidState);
            }
            membership[index] = 1;
        }

        for index in &state.free_indices {
            let index_usize =
                usize::try_from(*index).map_err(|_| WorldCheckpointError::IndexOverflow)?;
            let Some(slot) = state.slots.get(index_usize) else {
                return Err(WorldCheckpointError::InvalidState);
            };
            if slot.alive || membership[index_usize] != 0 {
                return Err(WorldCheckpointError::InvalidState);
            }
            membership[index_usize] = 2;
        }

        for (slot, membership) in state.slots.iter().zip(&membership) {
            if (slot.alive && *membership != 1) || (!slot.alive && *membership != 2) {
                return Err(WorldCheckpointError::InvalidState);
            }
        }

        let mut live_positions = Vec::new();
        live_positions
            .try_reserve_exact(slot_count)
            .map_err(|_| WorldCheckpointError::AllocationFailed)?;
        live_positions.resize(slot_count, None);
        for (position, entity) in state.live_entities.iter().copied().enumerate() {
            let index =
                usize::try_from(entity.index).map_err(|_| WorldCheckpointError::IndexOverflow)?;
            let encoded = position
                .checked_add(1)
                .and_then(std::num::NonZeroUsize::new)
                .ok_or(WorldCheckpointError::IndexOverflow)?;
            live_positions[index] = Some(encoded);
        }

        let mut slots = Vec::new();
        slots
            .try_reserve_exact(slot_count)
            .map_err(|_| WorldCheckpointError::AllocationFailed)?;
        for slot in state.slots {
            slots.push(EntitySlot {
                generation: slot.generation,
                alive: slot.alive,
            });
        }

        Ok(Self {
            slots,
            free_indices: state.free_indices,
            live_entities: state.live_entities,
            live_positions,
        })
    }
}

//! Experimental native checkpoint DTOs for sparse component stores.
//!
//! These values are not a byte format. Type registrations and payload codecs
//! remain supplied by trusted model code.

use crate::{ComponentStore, SparseEntry};
use kairo_ecs_types::EntityId;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;

const COMPONENT_CHECKPOINT_VERSION_V1: u32 = 1;

/// Bounds rows and sparse address space before checkpoint allocations or codecs.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentCheckpointLimits {
    pub max_rows: usize,
    pub max_sparse_slots: usize,
}

/// Owned version 1 sparse-store image with payloads chosen by the caller.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentStoreCheckpointV1<P> {
    pub version: u32,
    pub sparse_slots: usize,
    pub rows: Vec<(EntityId, P)>,
}

/// Structural failures are kept distinct from caller codec errors.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentCheckpointStructureError {
    UnsupportedVersion(u32),
    RowLimitExceeded,
    SparseSlotLimitExceeded,
    IndexOverflow,
    DuplicateEntity,
    AllocationFailed,
}

/// Errors from checkpoint structure validation or caller-supplied codecs.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentCheckpointError<E> {
    Structure(ComponentCheckpointStructureError),
    Codec(E),
}

impl Display for ComponentCheckpointStructureError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported component checkpoint version: {version}")
            }
            Self::RowLimitExceeded => f.write_str("component checkpoint row limit exceeded"),
            Self::SparseSlotLimitExceeded => {
                f.write_str("component checkpoint sparse-slot limit exceeded")
            }
            Self::IndexOverflow => f.write_str("component checkpoint index is out of range"),
            Self::DuplicateEntity => f.write_str("duplicate entity in component checkpoint"),
            Self::AllocationFailed => f.write_str("component checkpoint allocation failed"),
        }
    }
}

impl Error for ComponentCheckpointStructureError {}

impl<E: Display> Display for ComponentCheckpointError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Structure(error) => Display::fmt(error, f),
            Self::Codec(error) => write!(f, "component checkpoint codec failed: {error}"),
        }
    }
}

impl<E: Error + 'static> Error for ComponentCheckpointError<E> {}

fn reserve<T>(items: &mut Vec<T>, count: usize) -> Result<(), ComponentCheckpointStructureError> {
    items
        .try_reserve_exact(count)
        .map_err(|_| ComponentCheckpointStructureError::AllocationFailed)
}

impl<T> ComponentStore<T> {
    /// Encode borrowed dense values without requiring `T: Clone`.
    #[doc(hidden)]
    pub fn checkpoint_state_with<P, E>(
        &self,
        limits: ComponentCheckpointLimits,
        mut encode: impl FnMut(&T) -> Result<P, E>,
    ) -> Result<ComponentStoreCheckpointV1<P>, ComponentCheckpointError<E>> {
        if self.dense.len() > limits.max_rows {
            return Err(ComponentCheckpointError::Structure(
                ComponentCheckpointStructureError::RowLimitExceeded,
            ));
        }
        if self.sparse.len() > limits.max_sparse_slots {
            return Err(ComponentCheckpointError::Structure(
                ComponentCheckpointStructureError::SparseSlotLimitExceeded,
            ));
        }

        let mut rows = Vec::new();
        reserve(&mut rows, self.dense.len()).map_err(ComponentCheckpointError::Structure)?;
        for (entity, value) in self.entities.iter().copied().zip(&self.dense) {
            rows.push((
                entity,
                encode(value).map_err(ComponentCheckpointError::Codec)?,
            ));
        }
        Ok(ComponentStoreCheckpointV1 {
            version: COMPONENT_CHECKPOINT_VERSION_V1,
            sparse_slots: self.sparse.len(),
            rows,
        })
    }

    /// Validate the complete image before calling `decode`, then build a private store.
    #[doc(hidden)]
    pub fn from_checkpoint_state_with<P, E>(
        image: ComponentStoreCheckpointV1<P>,
        limits: ComponentCheckpointLimits,
        mut decode: impl FnMut(P) -> Result<T, E>,
    ) -> Result<Self, ComponentCheckpointError<E>> {
        let structure = |error| ComponentCheckpointError::Structure(error);
        if image.version != COMPONENT_CHECKPOINT_VERSION_V1 {
            return Err(structure(
                ComponentCheckpointStructureError::UnsupportedVersion(image.version),
            ));
        }
        if image.rows.len() > limits.max_rows {
            return Err(structure(
                ComponentCheckpointStructureError::RowLimitExceeded,
            ));
        }
        if image.sparse_slots > limits.max_sparse_slots {
            return Err(structure(
                ComponentCheckpointStructureError::SparseSlotLimitExceeded,
            ));
        }

        // A byte per allowed sparse slot gives bounded linear duplicate checks.
        let mut membership = Vec::new();
        reserve(&mut membership, image.sparse_slots).map_err(structure)?;
        membership.resize(image.sparse_slots, 0_u8);
        let mut indices = Vec::new();
        reserve(&mut indices, image.rows.len()).map_err(structure)?;
        for (entity, _) in &image.rows {
            let index = usize::try_from(entity.index)
                .map_err(|_| structure(ComponentCheckpointStructureError::IndexOverflow))?;
            let Some(seen) = membership.get_mut(index) else {
                return Err(structure(ComponentCheckpointStructureError::IndexOverflow));
            };
            if *seen != 0 {
                return Err(structure(
                    ComponentCheckpointStructureError::DuplicateEntity,
                ));
            }
            *seen = 1;
            indices.push(index);
        }

        // All structure is valid. Reserve destination vectors before invoking
        // any payload decoder; a codec failure still cannot expose this store.
        let mut sparse = Vec::new();
        reserve(&mut sparse, image.sparse_slots).map_err(structure)?;
        sparse.resize(image.sparse_slots, None);
        let mut dense = Vec::new();
        reserve(&mut dense, image.rows.len()).map_err(structure)?;
        let mut entities = Vec::new();
        reserve(&mut entities, image.rows.len()).map_err(structure)?;

        for (position, ((entity, payload), index)) in
            image.rows.into_iter().zip(indices).enumerate()
        {
            let position = position
                .checked_add(1)
                .and_then(NonZeroUsize::new)
                .ok_or_else(|| structure(ComponentCheckpointStructureError::IndexOverflow))?;
            let value = decode(payload).map_err(ComponentCheckpointError::Codec)?;
            sparse[index] = Some(SparseEntry {
                generation: entity.generation,
                position,
            });
            entities.push(entity);
            dense.push(value);
        }
        Ok(Self {
            dense,
            sparse,
            entities,
        })
    }
}

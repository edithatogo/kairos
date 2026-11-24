use std::{
    error::Error,
    fmt::{self, Display},
};

use kairo_ecs_types::EntityId;
use serde::{Deserialize, Serialize};

use crate::{encode_dense_position, EntitySlot, World};

const FORMAT_VERSION: u16 = 1;

/// Failure to encode or restore a durable entity-world checkpoint.
#[derive(Debug)]
pub enum WorldCheckpointError {
    /// The data could not be encoded or decoded as checkpoint JSON.
    Json(serde_json::Error),
    /// The decoder does not support the encoded version.
    UnsupportedVersion(u16),
    /// The free list does not describe a valid set of retired slots.
    InvalidState(&'static str),
}

impl Display for WorldCheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "world checkpoint JSON: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported world checkpoint JSON version {version}")
            }
            Self::InvalidState(reason) => write!(f, "invalid world checkpoint: {reason}"),
        }
    }
}

impl Error for WorldCheckpointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
struct CheckpointVersion {
    version: u16,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CheckpointV1 {
    version: u16,
    generations: Vec<u32>,
    free_indices: Vec<u64>,
}

impl World {
    /// Experimental Rust API: encode entity generations and free-slot order
    /// as deterministic versioned JSON.
    ///
    /// Capture model-owned component values at the same event boundary.
    /// The caller owns durable storage and checkpoint integrity.
    pub fn checkpoint_json(&self) -> Result<Vec<u8>, WorldCheckpointError> {
        serde_json::to_vec(&CheckpointV1 {
            version: FORMAT_VERSION,
            generations: self.slots.iter().map(|slot| slot.generation).collect(),
            free_indices: self.free_indices.clone(),
        })
        .map_err(WorldCheckpointError::Json)
    }

    /// Restore live entity handles and the deterministic allocation order
    /// from versioned JSON. Restore component values separately against the
    /// returned world's live handles.
    ///
    /// Returns an error for malformed JSON, unsupported versions or invalid
    /// free-slot indices.
    pub fn from_checkpoint_json(bytes: &[u8]) -> Result<Self, WorldCheckpointError> {
        let CheckpointVersion { version } =
            serde_json::from_slice(bytes).map_err(WorldCheckpointError::Json)?;
        if version != FORMAT_VERSION {
            return Err(WorldCheckpointError::UnsupportedVersion(version));
        }
        let CheckpointV1 {
            generations,
            free_indices,
            ..
        } = serde_json::from_slice(bytes).map_err(WorldCheckpointError::Json)?;

        let mut slots: Vec<_> = generations
            .into_iter()
            .map(|generation| EntitySlot {
                generation,
                alive: true,
            })
            .collect();
        for &index in &free_indices {
            let slot = usize::try_from(index)
                .ok()
                .and_then(|index| slots.get_mut(index))
                .ok_or(WorldCheckpointError::InvalidState(
                    "free slot index out of range",
                ))?;
            if !slot.alive {
                return Err(WorldCheckpointError::InvalidState(
                    "duplicate free slot index",
                ));
            }
            slot.alive = false;
        }

        let mut live_positions = vec![None; slots.len()];
        let mut live_entities = Vec::with_capacity(slots.len() - free_indices.len());
        for (index, slot) in slots.iter().enumerate().filter(|(_, slot)| slot.alive) {
            live_positions[index] = Some(encode_dense_position(live_entities.len()));
            live_entities.push(EntityId::new(index as u64, slot.generation));
        }
        Ok(Self {
            slots,
            free_indices,
            live_entities,
            live_positions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_restores_stale_handles_and_reuse_order() {
        let mut world = World::new();
        let agents: [_; 4] = std::array::from_fn(|_| world.spawn());
        assert!(world.despawn(agents[0]));
        assert!(world.despawn(agents[2]));

        let bytes = world.checkpoint_json().unwrap();
        let mut restored = World::from_checkpoint_json(&bytes).unwrap();
        assert_eq!(restored.checkpoint_json().unwrap(), bytes);
        assert_eq!(restored.snapshot(), world.snapshot());
        assert!(restored.is_alive(agents[3]));
        assert!(!restored.is_alive(agents[2]));
        for expected in [
            EntityId::new(2, 1),
            EntityId::new(0, 1),
            EntityId::new(4, 0),
        ] {
            assert_eq!(restored.spawn(), expected);
            assert_eq!(world.spawn(), expected);
        }
        assert!(!restored.despawn(agents[2]));
        assert!(restored.despawn(EntityId::new(2, 1)));
        assert_eq!(restored.spawn(), EntityId::new(2, 2));
    }

    #[test]
    fn checkpoint_rejects_an_invalid_slot_table() {
        let malformed = World::from_checkpoint_json(b"{").unwrap_err();
        assert!(malformed.to_string().starts_with("world checkpoint JSON:"));
        assert!(malformed.source().is_some());
        let unsupported = World::from_checkpoint_json(br#"{"version":2,"slots":[]}"#).unwrap_err();
        assert!(matches!(
            unsupported,
            WorldCheckpointError::UnsupportedVersion(2)
        ));
        assert_eq!(
            unsupported.to_string(),
            "unsupported world checkpoint JSON version 2"
        );

        for (bytes, problem) in [
            (
                &br#"{"version":1,"generations":[1],"free_indices":[1]}"#[..],
                "free slot index out of range",
            ),
            (
                &br#"{"version":1,"generations":[1],"free_indices":[0,0]}"#[..],
                "duplicate free slot index",
            ),
        ] {
            let error = World::from_checkpoint_json(bytes).unwrap_err();
            assert!(
                matches!(error, WorldCheckpointError::InvalidState(reason) if reason == problem)
            );
            assert!(error.source().is_none());
            assert_eq!(
                error.to_string(),
                format!("invalid world checkpoint: {problem}")
            );
        }
    }
}

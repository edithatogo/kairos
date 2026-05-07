#![forbid(unsafe_code)]

use kairo_ecs_types::EntityId;

/// Domain separator for run-seed mixing.
const RUN_SEED_DOMAIN: u64 = 0xA8E5_1B2C_4D6F_9013;

/// Domain separator for entity-index mixing.
const ENTITY_INDEX_DOMAIN: u64 = 0x9E37_79B9_7F4A_7C15;

/// Domain separator for entity-generation mixing.
const ENTITY_GENERATION_DOMAIN: u64 = 0xBF58_476D_1CE4_E5B9;

/// Multiply constant for entity-index mixing.
const ENTITY_INDEX_MIX: u64 = 0xD6E8_FEB8_6659_FD93;

/// Multiply constant for entity-generation mixing.
const ENTITY_GENERATION_MIX: u64 = 0x94D0_49BB_1331_11EB;

/// splitmix64 algorithm constants
const SPLITMIX64_GAMMA: u64 = 0x9E3779B97F4A7C15;
const SPLITMIX64_SHIFT1: u32 = 30;
const SPLITMIX64_MULT1: u64 = 0xBF58476D1CE4E5B9;
const SPLITMIX64_SHIFT2: u32 = 27;
const SPLITMIX64_MULT2: u64 = 0x94D049BB133111EB;
const SPLITMIX64_SHIFT3: u32 = 31;

/// Deterministic stream seed derived from a run seed and entity handle.
///
/// The same `run_seed` and `EntityId` always produce the same stream seed.
/// Distinct entity handles are mixed through independent domains so that
/// changes in either the index or generation diverge the derived stream.
pub fn derive_entity_seed(run_seed: u64, entity: EntityId) -> u64 {
    let mut state = run_seed.wrapping_add(RUN_SEED_DOMAIN);
    state ^= splitmix64(entity.index.wrapping_add(ENTITY_INDEX_DOMAIN));
    state = state.rotate_left(17);
    state = state.wrapping_add(
        splitmix64(u64::from(entity.generation).wrapping_add(ENTITY_GENERATION_DOMAIN))
            .wrapping_mul(ENTITY_GENERATION_MIX),
    );
    splitmix64(state ^ ENTITY_INDEX_MIX)
}

/// Convenience helper for deriving a reproducible stream for an entity.
pub fn entity_stream(run_seed: u64, entity: EntityId) -> DeterministicStream {
    DeterministicStream::from_seed(derive_entity_seed(run_seed, entity))
}

/// SplitMix64-backed deterministic stream.
///
/// `new` consumes the seed directly. Use `from_run_seed` for a canonical
/// explicit run-seed entrypoint and `from_entity` for child streams.
#[derive(Clone, Debug)]
pub struct DeterministicStream {
    state: u64,
}

impl DeterministicStream {
    /// Construct a stream from a raw seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Construct a stream from a run seed.
    pub fn from_run_seed(run_seed: u64) -> Self {
        Self::new(run_seed)
    }

    /// Construct a stream from a derived entity seed.
    pub fn from_seed(seed: u64) -> Self {
        Self::new(seed)
    }

    /// Construct a stream for a particular entity in a particular run.
    pub fn from_entity(run_seed: u64, entity: EntityId) -> Self {
        Self::from_seed(derive_entity_seed(run_seed, entity))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = splitmix64(self.state);
        self.state
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    pub fn into_inner(self) -> u64 {
        self.state
    }
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(SPLITMIX64_GAMMA);
    let mut z = x;
    z = (z ^ (z >> SPLITMIX64_SHIFT1)).wrapping_mul(SPLITMIX64_MULT1);
    z = (z ^ (z >> SPLITMIX64_SHIFT2)).wrapping_mul(SPLITMIX64_MULT2);
    z ^ (z >> SPLITMIX64_SHIFT3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_seed_is_reproducible() {
        let entity = EntityId {
            index: 42,
            generation: 0,
        };

        assert_eq!(derive_entity_seed(7, entity), derive_entity_seed(7, entity));
    }

    #[test]
    fn stream_replays_from_run_seed() {
        let mut a = DeterministicStream::from_run_seed(123);
        let mut b = DeterministicStream::from_run_seed(123);

        assert_eq!(a.next_u64(), b.next_u64());
        assert_eq!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn distinct_entity_inputs_diverge() {
        let first = EntityId {
            index: 1,
            generation: 0,
        };
        let second = EntityId {
            index: 2,
            generation: 0,
        };
        let third = EntityId {
            index: 1,
            generation: 1,
        };

        assert_ne!(
            derive_entity_seed(123, first),
            derive_entity_seed(123, second)
        );
        assert_ne!(
            derive_entity_seed(123, first),
            derive_entity_seed(123, third)
        );
    }

    #[test]
    fn entity_stream_replays_exactly() {
        let entity = EntityId {
            index: 99,
            generation: 4,
        };

        let mut first = DeterministicStream::from_entity(7, entity);
        let mut second = entity_stream(7, entity);

        assert_eq!(first.next_u64(), second.next_u64());
        assert_eq!(first.next_u32(), second.next_u32());
        assert_eq!(first.next_u64(), second.next_u64());
    }

    #[test]
    fn different_run_seeds_diverge() {
        let entity = EntityId {
            index: 1,
            generation: 0,
        };

        assert_ne!(
            derive_entity_seed(123, entity),
            derive_entity_seed(124, entity)
        );
    }
}

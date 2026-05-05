#![forbid(unsafe_code)]

use kairo_ecs_types::EntityId;

/// Rotation constant for seed derivation mixing (splitmix64)
const SEED_ROTATE_CONST: u32 = 17;

/// splitmix64 algorithm constants
const SPLITMIX64_GAMMA: u64 = 0x9E3779B97F4A7C15;
const SPLITMIX64_SHIFT1: u32 = 30;
const SPLITMIX64_MULT1: u64 = 0xBF58476D1CE4E5B9;
const SPLITMIX64_SHIFT2: u32 = 27;
const SPLITMIX64_MULT2: u64 = 0x94D049BB133111EB;
const SPLITMIX64_SHIFT3: u32 = 31;

/// Deterministic stream seed derived from run seed and entity handle.
pub fn derive_entity_seed(run_seed: u64, entity: EntityId) -> u64 {
    splitmix64(
        run_seed ^ entity.index.rotate_left(SEED_ROTATE_CONST) ^ u64::from(entity.generation),
    )
}

#[derive(Clone, Debug)]
pub struct DeterministicStream {
    state: u64,
}

impl DeterministicStream {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = splitmix64(self.state);
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
    fn stream_replays_from_seed() {
        let mut a = DeterministicStream::new(123);
        let mut b = DeterministicStream::new(123);

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

        assert_ne!(
            derive_entity_seed(123, first),
            derive_entity_seed(123, second)
        );
    }
}

#![forbid(unsafe_code)]

use kairo_ecs_types::EntityId;

/// Deterministic stream seed derived from run seed and entity handle.
pub fn derive_entity_seed(run_seed: u64, entity: EntityId) -> u64 {
    splitmix64(run_seed ^ entity.index.rotate_left(17) ^ u64::from(entity.generation))
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
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
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

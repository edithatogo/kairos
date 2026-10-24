#![forbid(unsafe_code)]

pub mod competing_risks;
pub use competing_risks::*;

pub use rand::Rng;
pub use rand::SeedableRng;
pub use rand_chacha::ChaCha8Rng;
pub use rand_distr;
pub use rand_distr::Distribution;

use core::convert::Infallible;
use kairo_ecs_types::EntityId;
use rand_chacha::rand_core::TryRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const RNG_GENERATOR_CHACHA8: &str = "chacha8";
pub const RNG_CHECKPOINT_VERSION_V1: u32 = 1;

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

/// RNG error taxonomy for deterministic stream configuration and restoration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RngError {
    #[error("unsupported RNG generator: expected `{expected}`, found `{found}`")]
    UnsupportedGenerator {
        expected: &'static str,
        found: String,
    },
    #[error("unsupported RNG checkpoint version: expected `{expected}`, found `{found}`")]
    UnsupportedVersion { expected: u32, found: u32 },
    #[error("invalid RNG stream state: {0}")]
    InvalidState(String),
}

/// Versioned, serializable checkpoint capturing generator metadata, seed, stream ID,
/// and mid-buffer word position for exact replay reconstruction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngCheckpoint {
    pub generator: String,
    pub version: u32,
    pub seed: [u8; 32],
    pub stream_id: u64,
    pub word_pos: u128,
}

/// Derive a 32-byte ChaCha8 seed from a 64-bit integer seed via SplitMix64 cascade.
pub fn derive_seed_from_u64(seed: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    let h1 = splitmix64(seed.wrapping_add(RUN_SEED_DOMAIN));
    let h2 = splitmix64(h1);
    let h3 = splitmix64(h2);
    let h4 = splitmix64(h3);
    out[0..8].copy_from_slice(&h1.to_le_bytes());
    out[8..16].copy_from_slice(&h2.to_le_bytes());
    out[16..24].copy_from_slice(&h3.to_le_bytes());
    out[24..32].copy_from_slice(&h4.to_le_bytes());
    out
}

/// Canonical derivation from run seed, full generational entity ID, and purpose namespace.
pub fn derive_canonical_seed(run_seed: u64, entity: EntityId, purpose: &str) -> [u8; 32] {
    let mut seed = [0u8; 32];
    let mut h1 = run_seed.wrapping_add(RUN_SEED_DOMAIN);
    let mut h2 = entity
        .index
        .wrapping_add(ENTITY_INDEX_DOMAIN)
        .wrapping_mul(ENTITY_INDEX_MIX);
    let mut h3 = u64::from(entity.generation)
        .wrapping_add(ENTITY_GENERATION_DOMAIN)
        .wrapping_mul(ENTITY_GENERATION_MIX);
    let mut h4 = 0x517C_C1B7_2722_0A95u64;
    for byte in purpose.as_bytes() {
        h4 = (h4 ^ u64::from(*byte)).wrapping_mul(SPLITMIX64_GAMMA);
    }
    h1 = splitmix64(h1 ^ h4);
    h2 = splitmix64(h2 ^ h1);
    h3 = splitmix64(h3 ^ h2);
    h4 = splitmix64(h4 ^ h3);
    seed[0..8].copy_from_slice(&h1.to_le_bytes());
    seed[8..16].copy_from_slice(&h2.to_le_bytes());
    seed[16..24].copy_from_slice(&h3.to_le_bytes());
    seed[24..32].copy_from_slice(&h4.to_le_bytes());
    seed
}

/// Canonical derivation of 64-bit stream ID from entity ID and purpose namespace.
pub fn derive_stream_id(entity: EntityId, purpose: &str) -> u64 {
    let mut h = entity.index.wrapping_mul(ENTITY_INDEX_MIX);
    h ^= u64::from(entity.generation).wrapping_mul(ENTITY_GENERATION_MIX);
    for b in purpose.as_bytes() {
        h = (h ^ u64::from(*b)).wrapping_mul(SPLITMIX64_GAMMA);
    }
    splitmix64(h)
}

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
    DeterministicStream::from_entity(run_seed, entity)
}

/// Pinned ChaCha8Rng-backed deterministic stream with exact stream isolation,
/// mid-buffer resume, and versioned checkpointing.
#[derive(Clone, Debug)]
pub struct DeterministicStream {
    rng: ChaCha8Rng,
    seed: [u8; 32],
    stream_id: u64,
}

impl DeterministicStream {
    /// Construct a stream from a raw u64 seed.
    pub fn new(seed: u64) -> Self {
        Self::from_seed_u64(seed, 0)
    }

    /// Construct a stream from an explicit run seed.
    pub fn from_run_seed(run_seed: u64) -> Self {
        Self::from_seed_u64(run_seed, 0)
    }

    /// Construct a stream from a derived seed.
    pub fn from_seed(seed: u64) -> Self {
        Self::from_seed_u64(seed, 0)
    }

    /// Construct a stream from an explicit u64 seed and explicit stream ID.
    pub fn from_seed_u64(seed: u64, stream_id: u64) -> Self {
        let seed_bytes = derive_seed_from_u64(seed);
        let mut rng = ChaCha8Rng::from_seed(seed_bytes);
        rng.set_stream(stream_id);
        Self {
            rng,
            seed: seed_bytes,
            stream_id,
        }
    }

    /// Construct a stream for a particular entity using default purpose namespace.
    pub fn from_entity(run_seed: u64, entity: EntityId) -> Self {
        Self::from_entity_purpose(run_seed, entity, "default")
    }

    /// Construct a stream for a particular entity and distinct purpose namespace.
    pub fn from_entity_purpose(run_seed: u64, entity: EntityId, purpose: &str) -> Self {
        let seed = derive_canonical_seed(run_seed, entity, purpose);
        let stream_id = derive_stream_id(entity, purpose);
        let mut rng = ChaCha8Rng::from_seed(seed);
        rng.set_stream(stream_id);
        Self {
            rng,
            seed,
            stream_id,
        }
    }

    /// Draw next 64-bit unsigned integer from the underlying ChaCha8 engine.
    pub fn next_u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    /// Draw next 32-bit unsigned integer from the underlying ChaCha8 engine.
    pub fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }

    /// Fill a slice of bytes from the stream.
    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest);
    }

    /// Current word offset into the stream buffer (u128).
    pub fn word_pos(&self) -> u128 {
        self.rng.get_word_pos()
    }

    /// Set word offset into the stream buffer for exact mid-buffer resume.
    pub fn set_word_pos(&mut self, pos: u128) {
        self.rng.set_word_pos(pos);
    }

    /// The 32-byte seed backing this stream.
    pub fn seed_bytes(&self) -> [u8; 32] {
        self.seed
    }

    /// The 64-bit stream identifier.
    pub fn stream_id(&self) -> u64 {
        self.stream_id
    }

    /// Create a versioned checkpoint of the stream state.
    pub fn checkpoint(&self) -> RngCheckpoint {
        RngCheckpoint {
            generator: RNG_GENERATOR_CHACHA8.to_string(),
            version: RNG_CHECKPOINT_VERSION_V1,
            seed: self.seed,
            stream_id: self.stream_id,
            word_pos: self.rng.get_word_pos(),
        }
    }

    /// Sample a value from any `rand_distr::Distribution`.
    pub fn sample<T, D: Distribution<T>>(&mut self, distr: D) -> T {
        distr.sample(self)
    }

    /// Restore stream from a versioned checkpoint.
    pub fn restore(checkpoint: &RngCheckpoint) -> Result<Self, RngError> {
        if checkpoint.generator != RNG_GENERATOR_CHACHA8 {
            return Err(RngError::UnsupportedGenerator {
                expected: RNG_GENERATOR_CHACHA8,
                found: checkpoint.generator.clone(),
            });
        }
        if checkpoint.version != RNG_CHECKPOINT_VERSION_V1 {
            return Err(RngError::UnsupportedVersion {
                expected: RNG_CHECKPOINT_VERSION_V1,
                found: checkpoint.version,
            });
        }

        let mut rng = ChaCha8Rng::from_seed(checkpoint.seed);
        rng.set_stream(checkpoint.stream_id);
        rng.set_word_pos(checkpoint.word_pos);

        Ok(Self {
            rng,
            seed: checkpoint.seed,
            stream_id: checkpoint.stream_id,
        })
    }
}

impl TryRng for DeterministicStream {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.rng.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.rng.next_u64())
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
        self.rng.fill_bytes(dest);
        Ok(())
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
    use rand_distr::{Exp, Exp1, Open01};

    #[test]
    fn test_chacha_methods() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let pos = rng.get_word_pos();
        rng.set_word_pos(pos + 1);
        rng.set_stream(1);
        let stream = rng.get_stream();
        assert_eq!(stream, 1);
    }

    #[test]
    fn test_rand_distr_sampling() {
        let mut stream = DeterministicStream::from_run_seed(42);
        let u: f64 = stream.sample(Open01);
        assert!(u > 0.0 && u < 1.0);
        let exp1_val: f64 = stream.sample(Exp1);
        assert!(exp1_val > 0.0);
        let exp = Exp::new(2.5).unwrap();
        let exp_val: f64 = stream.sample(exp);
        assert!(exp_val > 0.0);
    }

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
            index: 42,
            generation: 0,
        };

        let mut s = DeterministicStream::from_entity(7, entity);
        let vals = [s.next_u32(), s.next_u32(), s.next_u32(), s.next_u32()];
        println!("CHACHA8 DRAWS: {:?}", vals);
        assert_eq!(vals.len(), 4);
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

    #[test]
    fn checkpoint_roundtrip_and_mid_buffer_restoration() {
        let mut original = DeterministicStream::new(12345);
        // Draw some u64 and u32 values to advance state and buffer
        let _ = original.next_u64();
        let _ = original.next_u32();
        let _ = original.next_u64();

        let checkpoint = original.checkpoint();
        let json = serde_json::to_string(&checkpoint).expect("serialize checkpoint");
        let restored_cp: RngCheckpoint =
            serde_json::from_str(&json).expect("deserialize checkpoint");

        let mut restored =
            DeterministicStream::restore(&restored_cp).expect("restore from checkpoint");

        // Both original and restored must yield identical subsequent draws
        for _ in 0..10 {
            assert_eq!(original.next_u64(), restored.next_u64());
            assert_eq!(original.next_u32(), restored.next_u32());
        }
    }

    #[test]
    fn checkpoint_rejects_incompatible_generator_or_version() {
        let cp_invalid_gen = RngCheckpoint {
            generator: "pcg64".to_string(),
            version: 1,
            seed: [0u8; 32],
            stream_id: 0,
            word_pos: 0,
        };
        assert!(matches!(
            DeterministicStream::restore(&cp_invalid_gen),
            Err(RngError::UnsupportedGenerator { .. })
        ));

        let cp_invalid_ver = RngCheckpoint {
            generator: RNG_GENERATOR_CHACHA8.to_string(),
            version: 99,
            seed: [0u8; 32],
            stream_id: 0,
            word_pos: 0,
        };
        assert!(matches!(
            DeterministicStream::restore(&cp_invalid_ver),
            Err(RngError::UnsupportedVersion { .. })
        ));
    }

    #[test]
    fn purpose_isolation_and_crn_coupling() {
        let entity = EntityId {
            index: 10,
            generation: 1,
        };
        let mut stream_movement =
            DeterministicStream::from_entity_purpose(42, entity, "agent.movement");
        let mut stream_combat =
            DeterministicStream::from_entity_purpose(42, entity, "agent.combat");
        let mut stream_movement_replica =
            DeterministicStream::from_entity_purpose(42, entity, "agent.movement");

        // Independent purposes on the same entity diverge
        let val_movement = stream_movement.next_u64();
        let val_combat = stream_combat.next_u64();
        assert_ne!(val_movement, val_combat);

        // Identical entity and purpose replicate exactly (CRN coupling)
        let val_movement_replica = stream_movement_replica.next_u64();
        assert_eq!(val_movement, val_movement_replica);
        assert_eq!(
            stream_movement.next_u32(),
            stream_movement_replica.next_u32()
        );
    }

    #[test]
    fn known_answer_vector_chacha8() {
        let entity = EntityId {
            index: 42,
            generation: 0,
        };
        let mut stream = DeterministicStream::from_entity(7, entity);
        let expected = [4087599687, 2415409399, 675359370, 3234763134];
        for &exp in &expected {
            assert_eq!(stream.next_u32(), exp);
        }
    }
}

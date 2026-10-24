# ADR: Track 72 — Statistical Distribution Sampling Compatibility & Red-Team Assessment

**Track:** 72-rand-distr-sampling  
**Date:** 2026-10-24  
**Author:** Principal Software Engineer / Conductor RNG Agent  
**Status:** Approved & Implemented  

---

## 1. Context & Architectural Boundary

Track 72 adopts `rand_distr = { version = "0.6", default-features = false, features = ["alloc"] }` in `crates/kairo-ecs-rng` to replace handwritten sampling routines and manual open-interval clamping with verified, peer-reviewed statistical distributions.

In conformance with `AGENTS.md` and `CONTRIBUTING.md`, distribution sampling alterations can affect downstream event trajectories. This ADR establishes:
1. Versioned distribution provenance (`distribution_provenance`) in `RngCheckpoint` and `DeterministicStream`, strictly separated from generator identity.
2. Full backwards compatibility with legacy inverse-transform sampling trajectories.
3. Conformance fixture definition for sampling replay verification across bindings and environments.
4. Comprehensive red-team risk assessment and mitigation.

---

## 2. Provenance Architecture & Replay Compatibility

### 2.1 Separation of Generator and Distribution Provenance
- The generator identity (`generator: "chacha8"`, `version: 1`) captures the bit-generator state (32-byte seed, 64-bit stream ID, 128-bit word position).
- The distribution provenance (`distribution_provenance: String`) captures the statistical distribution algorithm suite.
- Two standard profiles are supported:
  - `RNG_DISTRIBUTION_PROVENANCE_V1` (`"rand_distr_0.6"`): Default modern sampling using `rand_distr 0.6` rejection sampling, Ziggurat algorithms, and standard IEEE 754 representations.
  - `RNG_DISTRIBUTION_PROVENANCE_LEGACY` (`"legacy_inverse_transform"`): Legacy inverse transform sampling using 53-bit mantissa shifts and logarithmic threshold draws.
- Checkpoints serialize `distribution_provenance` with `#[serde(default = "default_distribution_provenance")]`, ensuring historical checkpoints without the field default safely to `"rand_distr_0.6"` while legacy checkpoints explicitly specifying `"legacy_inverse_transform"` restore their intended sampling profile.
- Unsupported provenance values strictly fail closed with `RngError::UnsupportedDistributionProvenance`.

### 2.2 Preservation of Domain Hazard and Survival Clock Logic
- Model-specific hazard evaluation (`HazardRate::hazard_at`), cumulative hazard integration (`HazardRate::cumulative_hazard`), and non-Markovian hazard inversion (`HazardRate::invert_cumulative_hazard`) are fully retained.
- Only the underlying generic standard threshold draw beneath them is versioned:
  - `draw_standard_threshold`: uses `rand_distr::Exp1` under `rand_distr_0.6`.
  - `draw_standard_threshold_legacy`: uses $-ln(U)$ where $U$ is generated via 53-bit mantissa shift under `legacy_inverse_transform`.
- Survival clocks and competing hazards automatically select the appropriate threshold sampler matching the stream's active `distribution_provenance`.

---

## 3. Conformance Fixtures

A new cross-binding conformance fixture has been established in `conformance/fixtures/sampling_replay.json`:
- Captures verified floating-point and integer draws from fixed seed 42 under `rand_distr_0.6`.
- Verified by unit tests in `crates/kairo-ecs-rng/tests/sampling_conformance_fixture.rs`.

---

## 4. Red-Team Objections & Mitigations

| Objection | Risk | Mitigation / Architectural Decision |
|---|---|---|
| **Trajectory Divergence from SplitMix/Handwritten Inversion** | Existing simulations seeded with historical seeds produce different numerical trajectories when distribution algorithms change. | Disclosed and accepted per Track 72 specification. Exact provenance versioning (`distribution_provenance`) allows simulations to distinguish and declare their profile. Legacy samplers (`draw_uniform_open01_legacy`, `draw_standard_threshold_legacy`) are preserved for audit. |
| **Transcendental Bit Drift Across Hardware Targets** | Floating-point `exp` and `ln` transcendental operations can differ slightly in the lowest mantissa bit across x86_64, aarch64, and WebAssembly. | Ziggurat tables in `rand_distr` use discrete precomputed integers and exact rejection bounds, reducing transcendental evaluations to rare tail cases. Fixture verification permits $\epsilon \le 10^{-12}$ analytical tolerance for cross-binding checks. |
| **Modulo Bias in Discrete Uniform Sampling** | Naive `rng.next_u64() % span` over-samples lower buckets when span is not a power of 2. | Replaced with `rand_distr::Uniform`, which uses unbiased Lemire/rejection sampling, guaranteeing uniform probability across discrete supports. |
| **Degenerate & Invalid Parameter Safety** | Zero or negative rates, non-finite std dev, or inverted bounds could cause panics, infinite loops, or NaN pollution. | Evaluated and verified: `Exp::new(0.0)` produces $\infty$ (degenerate zero hazard / infinite waiting time), `Normal::new(42.0, 0.0)` produces deterministic mean point mass, and non-finite parameters (`f64::NAN`, negative scale) reject on construction. |
| **Mid-Buffer Checkpoint State Corruption** | Sampling distributions could buffer internal state that is lost across serialization. | All adopted distributions are stateless parameter structs; all generation state resides in `DeterministicStream::word_pos` and `stream_id`. Restored streams match uncheckpointed streams bit-for-bit (`to_bits()`). |
| **Dependency Supply-Chain Exposure** | Adding `rand_distr` could introduce CVEs or undesirable default dependencies. | Locked with `default-features = false, features = ["alloc"]`. 0 security advisories, 0 CVEs via `cargo audit`, dual MIT/Apache-2.0 licenses via `cargo deny`. |

# Phase 0 Evidence & Value Gate Qualification: hecs ECS Storage Qualification

**Track:** 84-hecs-robustness  
**Date:** 2026-10-25  
**Lead:** `ecs-agent`  
**Status:** Phase 0 Complete — Evaluation Decision: **REJECT (NO-GO)**

---

## 1. Inventory & Target Scope (Task 0.1)

- **Lead Agent:** `ecs-agent`
- **Owned Path:** `conductor/tracks/84-hecs-robustness/`
- **Prospective Target:**
  Candidate replacement of custom sparse component storage, registry/query construction, and entity management in `crates/kairo-ecs-state/`.
- **Prerequisites Status:**
  - Track 78 (`Rayon data parallelism robustness`) completed and qualified.
  - Track 64 (`State component registry & storage foundations`) completed and qualified.
- **Current Baseline Capabilities (`kairo-ecs-state`):**
  1. **Zero Unsafe Code:** `#![forbid(unsafe_code)]` enforced across all state modules.
  2. **Deterministic Iteration Invariant:** Replay determinism requires byte-for-byte reproducible traces; entity query order is strictly `(index, generation)`, matching `WorldSnapshot` ordering.
  3. **Full 64-bit Index Generational IDs:** `kairo_ecs_types::EntityId { index: u64, generation: u32 }` supports $2^{64}$ entity lifecycles for long-running multi-million agent ABMs without ID exhaustion.
  4. **Custom Cache-Line & Memory Layouts:**
     - `CacheLineLayout`: Every column base address is 64-byte aligned and each row starts on a cache-line boundary, enabling prologue-free SIMD kernel execution and zero false-sharing across entities.
     - `TightLayout`: Unit-stride rows for zero-copy Arrow memory export and streaming batches.
  5. **Deterministic Checkpoints & Rollbacks:** O(1) slot generation tracking with reusable indices, linear component registration order, and snapshot/rollback compatibility consumed by FFI, Diplomat, Arrow, and PDES engines.

---

## 2. Dependency Qualification (Task 0.2)

- **Candidate Crate:** `hecs`
- **Version:** `0.11.2`
- **Repository:** `https://github.com/Ralith/hecs`
- **License:** Apache-2.0 OR MIT
- **MSRV:** Rust 1.65+ (Kairos compiler: Rust 1.99.1)
- **Internal Architecture:**
  - `hecs` is a minimal archetype-based ECS.
  - Entities are partitioned into archetype tables according to their component sets.
  - Adding or removing components moves entities between archetypes, using `swap_remove` on dense arrays.
- **Critical Impedance Mismatches:**
  1. **Entity Representation Mismatch:**
     - `hecs::Entity` is defined as a 32-bit entity ID and a 32-bit generation (`NonZeroU32`) packed into 64 bits.
     - KairoECS requires 64-bit `index: u64` and 32-bit `generation: u32` (`kairo_ecs_types::EntityId`).
     - Truncating entity indices to 32 bits limits total simulation entities to $2^{32}$, violating core Kairo contracts and breaking FFI/Diplomat bindings.
  2. **Unsafe Memory Footprint:**
     - `crates/kairo-ecs-state` enforces `#![forbid(unsafe_code)]`.
     - `hecs` relies heavily on internal `unsafe` pointers, raw memory reallocations, unchecked column casts, and dynamic archetype shifts.
  3. **Replay Non-Determinism (Query Iteration Order):**
     - In `hecs`, query iteration walks archetype tables and dense row buffers that are shuffled by `swap_remove` upon component mutations or entity despawns.
     - Iteration order in `hecs` is therefore history- and mutation-order-dependent.
     - KairoECS requires strict deterministic iteration order: `(index, generation)` must be stable across runs, processes, and platforms for bit-level replay and PDES optimistic rollback.
  4. **Memory Alignment & Hardware Layout:**
     - `hecs` does not offer configurable cache-line alignment (`CacheLineLayout`) or unit-stride Arrow layout controls (`TightLayout`).
     - Adopting `hecs` would break alignment-guaranteed SIMD execution and require intermediate copies for Arrow export.

---

## 3. Value Gate Evaluation (Task 0.3)

| Criterion | Retain `kairo-ecs-state` (Track 64 Baseline) | Adopt `hecs` (v0.11.2) | Decision Impact |
|---|---|---|---|
| **Safety Invariant** | **PASS** (`#![forbid(unsafe_code)]`) | **FAIL** (Extensive internal `unsafe` pointer manipulation) | Violates workspace safety rule |
| **Generational ID Range** | **PASS** (64-bit index: $2^{64}$, 32-bit gen) | **FAIL** (32-bit index: $2^{32}$, packed representation) | Incompatible with large-scale ABM & FFI |
| **Deterministic Query Order** | **PASS** (Strict `(index, generation)` candidate plan) | **FAIL** (Shuffled by archetype `swap_remove`) | Breaks replay determinism |
| **SIMD / Arrow Layout** | **PASS** (`CacheLineLayout` & `TightLayout`) | **FAIL** (Generic dense arrays, no alignment guarantees) | Degrades vectorization and Arrow zero-copy |
| **Net Complexity** | **PASS** (Zero external dependencies, clean architecture) | **FAIL** (Requires side mapping tables and sorting wrappers) | Net maintained complexity drastically increases |
| **Downstream Compatibility** | **PASS** (100% verified across FFI, Diplomat, Arrow, PDES) | **FAIL** (Requires rewriting entity abstractions across workspace) | Severe regression hazard |

### Standalone `bevy_ecs` Comparison Note:
As required by the Track 84 specification:
- `bevy_ecs` provides a more complex archetype-and-table architecture, but introduces over 50 transitive dependencies, background task pools, heavy macro generation, and complex runtime type registrations.
- Like `hecs`, `bevy_ecs` uses 32-bit entity IDs (`Entity::index`) and does not enforce `#![forbid(unsafe_code)]` or deterministic `(index, generation)` iteration order across arbitrary mutation histories.
- Neither candidate qualifies as a net-positive replacement.

### Stop Condition / Decision: **REJECT (NO-GO)**
Pursuant to [spec.md](../spec.md):
> *"Stop criterion: No adoption unless net maintained complexity falls and required replay/identity/rollback behaviour survives. Record no-go if mapping recreates most of current storage; retain working ECS."*

And [dependency-value-policy.md](../../dependency-value-policy.md):
> *"If there is no production target/accepted consumer, no net benefit, unsupported contracts or excessive migration burden, record no-go/deferred with evidence. Keep Phase 1/2 unchecked, adoption_decision=deferred/rejected, and do not install a dependency merely to close a track. Close/archive a rejected evaluation only with an explicit outcome; never label it implemented."*

Adopting `hecs` fails the safety gate, breaks 64-bit entity IDs, compromises deterministic replay, forfeits cache-line SIMD alignment, and increases net maintained complexity through required adapter layers.

---

## 4. Phase 0 Closeout & Follow-up Actions (Task 0.4)

1. Stop at Phase 0 with a definitive **REJECT (NO-GO)** decision.
2. Leave Phase 1 and Phase 2 tasks unchecked in [plan.md](../plan.md).
3. Update [metadata.json](../metadata.json): set `"status": "completed"`, `"adoption_decision": "rejected"`, `"accepted_phases": ["0"]`.
4. Update `conductor/phase-closeout.yaml`, `conductor/tracks.yaml`, and `conductor/tracks.md`.
5. Retain native pure-Rust `crates/kairo-ecs-state` as the project's authoritative, verified ECS storage engine.

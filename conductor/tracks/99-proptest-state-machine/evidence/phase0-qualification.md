# Phase 0 Evidence & Value Gate Qualification: proptest-state-machine

**Track:** 99-proptest-state-machine  
**Date:** 2026-10-24  
**Agent:** conformance-agent  
**Status:** Phase 0 Complete — Evaluation Decision: **REJECT (NO-GO)**

---

## 1. Inventory & Target Scope (Task 0.1)

- **Lead Agent:** `conformance-agent`
- **Owned Path:** `conductor/tracks/99-proptest-state-machine/`
- **Prospective Target:**
  Repeated generated operation-sequence and shrinking scaffolding for state machine transitions in `crates/kairo-ecs-core` (Scheduler scheduling, cancellation, stepping, and conservation).
- **Prerequisites Status:**
  Track 96 (`proptest robustness`) is completed and qualified (`proptest = { version = "1.6", default-features = false, features = ["std"] }`, resolved to v1.12.0 in `Cargo.lock`).
- **Baseline Capabilities from Track 96:**
  Track 96 demonstrated that plain `proptest` already provides:
  1. Lexicographical total ordering property verification `(SimTime, priority, insertion_sequence)`.
  2. Step-wise and global event conservation: `scheduled = dispatched + cancelled + pending`.
  3. Idempotent cancellation and zero-dispatch for cancelled events.
  4. Full generational ID coverage across 64-bit indices and 32-bit generations.
  5. Deterministic replay via `RngSeed::Fixed` and bounded minimal counterexample shrinking.
  6. Negative controls detecting simulated dropped events and priority inversions.

---

## 2. Dependency Qualification (Task 0.2)

- **Candidate Crate:** `proptest-state-machine`
- **Version:** `0.9.0`
- **Repository:** `https://github.com/proptest-rs/proptest`
- **License:** MIT OR Apache-2.0
- **MSRV:** Rust 1.88+ (Kairos workspace compiler: Rust 1.99.1)
- **Dependency Footprint & Transitive Inflation:**
  `proptest-state-machine` declares:
  ```toml
  [dependencies.proptest]
  version = "1.12.0"
  features = ["fork", "timeout", "bit-set"]
  default-features = true
  ```
  - **Severe Transitive Bloat:** Unlike `kairo-ecs-core`'s locked minimal proptest configuration (`default-features = false, features = ["std"]`), `proptest-state-machine` forces on:
    - `fork`: Pulls in POSIX `nix`, `libc`, process spawning and signal handling.
    - `timeout`: Pulls in background timer polling threads.
    - `tempfile`, `regex-syntax`, `lazy_static`, `quick-error`.
  - **Non-Determinism & Portability Hazard:** Process forking and signal-based timeouts violate Kairo's strict deterministic replay contracts and break compatibility with WASM (`wasm32-unknown-unknown`) and non-POSIX environments.

---

## 3. Value Gate Evaluation (Task 0.3)

| Criterion | Retain Plain `proptest` (Track 96) | Adopt `proptest-state-machine` | Decision Impact |
|---|---|---|---|
| **Deterministic Replay** | **PASS** (`RngSeed::Fixed`, zero external threads/forks) | **FAIL** (Requires `fork` and `timeout` features) | Plain proptest strictly preserves DES replay |
| **Transitive Footprint** | **PASS** (Zero default features; minimal `std`) | **FAIL** (Pulls in `nix`, `tempfile`, `regex-syntax`, etc.) | Unacceptable bloat |
| **Code Simplicity** | **PASS** (Standard Rust enums, `vec(strategy, range)`, match loops) | **FAIL** (Requires `ReferenceStateMachine` + `StateMachineTest` traits, `BoxedStrategy`, and `prop_state_machine!` macro) | Adds ~70 lines boilerplate per state machine |
| **Shrinking Fidelity** | **PASS** (Native proptest shrinks minimal failing sequences) | **PASS** (Shrinks minimal transition sequence) | Parity; no gain |
| **WASM / Embedded Compat** | **PASS** (Clean compilation without OS dependencies) | **FAIL** (`fork` feature breaks non-POSIX targets) | Incompatible |
| **Maintenance Activity** | **PASS** (Mainline `proptest` active and widely tested) | **CONCERN** (Secondary experimental crate, low adoption) | Plain proptest is superior |

### Stop Condition / Decision: **REJECT (NO-GO)**
Pursuant to [dependency-value-policy.md](../../dependency-value-policy.md):
> *"If there is no production target/accepted consumer, no net benefit, unsupported contracts or excessive migration burden, record no-go/deferred with evidence. Keep Phase 1/2 unchecked, adoption_decision=deferred/rejected, and do not install a dependency merely to close a track. Close/archive a rejected evaluation only with an explicit outcome; never label it implemented."*

Adopting `proptest-state-machine` offers no net benefit over plain `proptest`, while adding substantial transitive dependencies (`fork`, `timeout`, `nix`) that compromise determinism and cross-platform compatibility.

---

## 4. Phase 0 Closeout & Follow-up Actions

1. Stop at Phase 0 with a definitive **REJECT** decision.
2. Leave Phase 1 and Phase 2 tasks unchecked in `plan.md`.
3. Update `metadata.json`: set `"adoption_decision": "rejected"`.
4. State machine testing for KairoECS remains authoritatively implemented with plain `proptest` via Track 96.

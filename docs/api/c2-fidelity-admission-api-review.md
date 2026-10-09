# API Review: C2 experimental fidelity and admission surface

## Problem and affected surface

The production calibration bridge needs the engine-owned fidelity policy and
admission permit. The affected root is `crates/kairo-ecs-des`, Rust API family
`kairo_ecs_des::fidelity`. This change makes an existing module public; it adds
no methods or changes to existing signatures or behavior.

## Proposed and implemented Rust API

The preview exposes the existing public items under `kairo_ecs_des::fidelity`:

- `FidelityMode::{Macro, Micro}`, `FidelityScope::{EntitySubsystem, Entity,
  Subsystem, Global}`, `FidelityError`, and copyable `FidelityDecision`.
- `FidelityPolicy::new(version, global)`, `set_entity`, `set_subsystem`,
  `set_entity_subsystem`, and `resolve`.
- `FidelityAdapter::new`, `admit`, `decision`, `stage_policy`,
  `apply_at_boundary`, and `prepare_admission`.
- Non-cloneable `FidelityAdmissionPermit::decision` and `bind`.

The exact signatures made reachable by this visibility-only change are:

```rust
pub enum FidelityMode { Macro, Micro }
pub enum FidelityScope { EntitySubsystem, Entity, Subsystem, Global }
pub enum FidelityError {
    UnsupportedVersion(u32), InvalidSubsystem, MissingPolicy, InvalidWork,
    DuplicateAdmission, BusyBoundary, NoPendingPolicy,
}
pub struct FidelityDecision {
    pub mode: FidelityMode,
    pub scope: FidelityScope,
    pub policy_version: u32,
}

impl FidelityPolicy {
    pub fn new(version: u32, global: Option<FidelityMode>) -> Result<Self, FidelityError>;
    pub fn set_entity(&mut self, entity: EntityId, mode: FidelityMode) -> Result<(), FidelityError>;
    pub fn set_subsystem(&mut self, subsystem: &str, mode: FidelityMode) -> Result<(), FidelityError>;
    pub fn set_entity_subsystem(&mut self, entity: EntityId, subsystem: &str, mode: FidelityMode) -> Result<(), FidelityError>;
    pub fn resolve(&self, entity: EntityId, subsystem: &str) -> Result<FidelityDecision, FidelityError>;
}

impl FidelityAdapter {
    pub fn new(policy: FidelityPolicy) -> Self;
    pub fn admit(&mut self, flow: &FlowRuntime, work: WorkId, subsystem: &str) -> Result<FidelityDecision, FidelityError>;
    pub fn prepare_admission<'a>(&'a mut self, flow: &FlowRuntime, owner: EntityId, subsystem: &str) -> Result<FidelityAdmissionPermit<'a>, FidelityError>;
    pub fn decision(&self, work: WorkId) -> Option<&FidelityDecision>;
    pub fn stage_policy(&mut self, policy: FidelityPolicy);
    pub fn apply_at_boundary(&mut self, flow: &FlowRuntime) -> Result<(), FidelityError>;
}

impl FidelityAdmissionPermit<'_> {
    pub fn decision(&self) -> FidelityDecision;
    pub fn bind(self, flow: &FlowRuntime, work: WorkId, expected: SimDuration)
        -> Result<FidelityDecision, (Self, FidelityError)>;
}
```

The bounds `FidelityAdapter` and `FidelityAdmissionPermit` are opaque outside
the module. `FidelityAdapter` implements `Clone` only under `cfg(test)`;
production consumers cannot clone it. No type is re-exported at the crate root.

## Compatibility and migration

| Surface | Change | Breaking for existing consumers? | Status |
|---|---|---:|---|
| Rust `kairo-ecs-des` | Additive visibility of `fidelity` and its existing public items | No | Experimental preview; future changes may break |
| C ABI | None | No | Unchanged |
| Arrow schema | None | No | Unchanged |
| Python, R, Julia, TypeScript, C#, Go | None | No | Unchanged |
| Serialization/checkpoint format | None | No | Unchanged |

No migration is needed by existing consumers. This review establishes no stable
API baseline, semver promise, portable checkpoint format, or clinical claim.
Track 25/D4 remains responsible for stable release qualification.

## Ownership, errors, and determinism

The permit exclusively borrows `FidelityAdapter` mutably until binding or drop,
so policy changes cannot race resolution. It captures private runtime identity,
owner, and decision state; it is not cloneable, serializable, or checkpoint data.
`bind` checks the actual runtime, live actor, duplicate admission, work owner,
original duration, and Pending state before mutating the adapter. Failure returns
the same permit with `FidelityError` for retry. Dropping it cancels the staged
admission without binding. `FidelityAdapter::decision(WorkId)` is adapter-local
and does not attest that a potentially colliding ID came from a given runtime.

Policy replacement checks all work recorded by this adapter. It is not a global
Flow quiescence guarantee for work created outside the adapter. Unknown or
despawned bindings fail closed; lifecycle cleanup and checkpoint rebinding need
separate reviewed contracts. The API changes no scheduler ordering, RNG stream,
event, telemetry, or checkpoint bytes. The adapter is not cloneable in production.

## Conformance and independent objections

`crates/kairo-ecs-des/tests/fidelity_public_api.rs` is compiled as an integration
consumer. It imports the module through the crate root, checks policy precedence,
and prepares/binds actual Pending Flow work through the public permit surface.

Independent review on 2026-10-06 by the read-only `api_gate_review` agent found
no blocking design objection to this exact additive experimental surface. The
review identified and disposed of these objections:

1. **Forged or colliding `WorkId`:** permit captures runtime identity and checks
   the actual work, owner, duration, and Pending state before binding.
2. **Policy mutation between resolution and binding:** the permit holds the
   adapter's exclusive mutable borrow; dropping the permit does not bind.
3. **Decision lookup mistaken for provenance:** document `decision` as
   adapter-local; copied decisions are descriptive metadata only.
4. **Unknown/despawned work strands policy updates:** fail closed; no cleanup or
   rebinding behavior is introduced by this export.
5. **Quiescence overstated:** checks cover only adapter-admitted work, not every
   Flow task created by other callers.
6. **Copied decision forged as authority:** binding consumes the private permit's
   captured decision, not a caller-supplied `FidelityDecision`.
7. **Stale generational entity configuration:** it can be inert configuration;
   admission validates the actual live actor and matching generation.

## Decision

Accepted for local experimental Rust preview after the exact-head public-import
test, compatibility review, documentation, and strict native checks pass. This
does not authorize a stable release or close C2.2/C2.1. Hosted exact-head checks
and reviewed parent integration remain separate gates.

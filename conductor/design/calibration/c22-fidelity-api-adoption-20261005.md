# C2.2 fidelity experimental API adoption decision

Coordinator decision: approve the exact additive experimental surface below,
subject to the explicit serial implementation gates. This is source-design
approval under the user's Kairos ownership authority, not runtime acceptance or
a stable semver baseline. Source reviewed at `2f69a3929706f4e4966e0b0a2ed5b7f6851a0b87`.

## Existing declarations to export

Change only `mod fidelity` to `pub mod fidelity` after both original mode and
lineage suites are migrated in-crate, preserving all helpers/assertions and
reconciling every named test once. Export through `kairo_ecs_des::fidelity`;
no additional root aliases are required. Types: FidelityMode, FidelityScope,
FidelityError, FidelityDecision, FidelityPolicy and FidelityAdapter. Preserve
exact fields, variants, methods and traits in the [independent review](../../evidence/c2.2-policy-api-20261005/review.json).
FidelityAdapter stays non-Clone in production; test-only Clone supports state
snapshot assertions. Internal fields remain private.

Approved existing method signatures:

```rust
FidelityPolicy::new(version: u32, global: Option<FidelityMode>) -> Result<FidelityPolicy, FidelityError>
FidelityPolicy::set_entity(&mut self, entity: EntityId, mode: FidelityMode) -> Result<(), FidelityError>
FidelityPolicy::set_subsystem(&mut self, subsystem: &str, mode: FidelityMode) -> Result<(), FidelityError>
FidelityPolicy::set_entity_subsystem(&mut self, entity: EntityId, subsystem: &str, mode: FidelityMode) -> Result<(), FidelityError>
FidelityPolicy::resolve(&self, entity: EntityId, subsystem: &str) -> Result<FidelityDecision, FidelityError>
FidelityAdapter::new(policy: FidelityPolicy) -> FidelityAdapter
FidelityAdapter::admit(&mut self, flow: &FlowRuntime, work: WorkId, subsystem: &str) -> Result<FidelityDecision, FidelityError>
FidelityAdapter::decision(&self, work: WorkId) -> Option<&FidelityDecision>
FidelityAdapter::stage_policy(&mut self, policy: FidelityPolicy) -> ()
FidelityAdapter::apply_at_boundary(&mut self, flow: &FlowRuntime) -> Result<(), FidelityError>
```

## Later borrowing admission permit

Only after test migration and external production API smoke tests, implement the
separately frozen permit:

```rust
pub struct FidelityAdmissionPermit<'a> { /* private, non-Clone */ }
impl FidelityAdapter { pub fn prepare_admission<'a>(&'a mut self, flow: &FlowRuntime, owner: EntityId, subsystem: &str) -> Result<FidelityAdmissionPermit<'a>, FidelityError>; }
impl<'a> FidelityAdmissionPermit<'a> { pub fn decision(&self) -> FidelityDecision; pub fn bind(self, flow: &FlowRuntime, work: WorkId, expected: SimDuration) -> Result<FidelityDecision, (Self, FidelityError)>; }
```

The mutable borrow freezes policy before sampling and real work creation.
Preparation validates actual actor/runtime before policy resolution without
binding lineage on failure. Bind checks actual Pending work, owner, runtime and
original duration; failure is atomic and returns the retryable permit. Bind
lineage only on successful admission. No duplicated seed state belongs in DES.
Existing admit and boundary behavior remain covered and compatible.
After permit implementation, require a second external production-module smoke
for prepare_admission, decision and bind before calibration bridge integration;
the earlier existing-API smoke does not qualify newly added permit methods.

## Review findings and dispositions

- Existing integration fixtures path-include private source and need test-only
  Clone. Migrate both into crate-root private cfg(test) modules before exports;
  never expose production Clone or weaken state oracles.
- Reuse the existing actor validator with crate-private visibility when needed
  for sibling permit code. Do not add a public actor helper.
- FlowRuntimeIdentity and FlowRuntime::identity are already public with an
  opaque private representation. Preserve both; do not narrow their visibility.
- The independent review found no architecture objection conditional on these
  gates. Corrected its original visibility statement before this decision.

## Compatibility and verification gates

Default DES/ABM Rust1.76 and canonical Rust1.99 remain mandatory. Optional
calibration integration has its separate1.88 floor. No dependency, manifest,
lockfile or DES-to-calibration edge is authorized by this decision. Required
source/API diff review, actual native tests, strict clippy, formatting,
production-module smoke and hosted exact-head CI precede implementation
acceptance. Do not infer any passed check from this design document.

The actual calibration Flow bridge and transit integration remain later bounded
work. C2.2 owned stream/resume, C2.1 paired execution, Track22 portable checkpoints,
ED MVP and stable release acceptance remain open.

# C2 execution admission v1 — reviewed test-first contract

Track03 owns mode resolution and Flow work boundaries. Track21 owns intrinsic
work-duration providers and the accepted Track01 purpose seed map. Track22 owns
portable continuation. This contract freezes the C2.1 test interfaces; it does
not claim C2 runtime, public-API acceptance, spatial routing or ED validity.

## Separate axes

Execution mode is Macro/Micro. Replay policy remains FreeRunning/ShadowAnchored.
Never alias those enums or derive one from the other. Do not introduce clinical
strata, site policies, external IO, dependencies or scheduler changes here.

## Mode resolver and boundaries

New Track03 source `crates/kairo-ecs-des/src/fidelity.rs` initially remains a
private experimental module; tests may include it by path. Public exports require
ADR, conformance/docs/compatibility and objection review before acceptance.

Freeze these test interfaces:

- `FidelityMode::{Macro, Micro}` and `FidelityScope::{EntitySubsystem, Entity,
  Subsystem, Global}`; both Copy/Debug/Eq.
- `FidelityPolicy::new(version: u32, global: Option<FidelityMode>) -> Result<Self,
  FidelityError>` accepts only version1. `set_entity(EntityId, mode)`,
  `set_subsystem(&str, mode)` and `set_entity_subsystem(EntityId, &str, mode)`
  return Result; IDs are real generational identities and subsystem IDs are
  nonempty, <=1024 UTF8bytes, no controls or boundary whitespace. No global mode
  is invented when unset. Exact byte identity and deterministic BTreeMap lookup.
- `resolve(EntityId, &str) -> Result<FidelityDecision, FidelityError>` uses exact
  pair, then entity, subsystem, global. Decision exposes mode/scope/policy_version.
- `FidelityAdapter::new(policy)` owns the policy, pending replacement and every
  admitted work binding. `admit(&FlowRuntime, WorkId, &str) -> Result<Decision,
  FidelityError>` validates the actual Flow work, requires Pending state, derives
  its generational owner, resolves once, and rejects duplicate work admission.
  A failed operation leaves bindings/policy/work unchanged.
- `decision(WorkId) -> Option<&FidelityDecision>` returns immutable admitted mode.
  Its identity does not change when policy is staged, work resumes or restarts.
- `stage_policy(new_policy)` stages one future policy, replacing only the pending
  configuration; it never changes admitted decisions or actual Flow work.
- `apply_at_boundary(&FlowRuntime) -> Result<(), FidelityError>` checks ALL bound
  Flow work (no caller-selected subset): Pending/Active/Suspended blocks with
  BusyBoundary. Unknown/despawned work fails closed; no policy change, work
  cancellation, context loss or resource reassignment may force a boundary.
  Completed/Aborted/Cancelled/Released are terminal. With all work terminal and
  a pending policy, apply for future admissions only. NoPendingPolicy if absent.
- Error cases include UnsupportedVersion, InvalidSubsystem, MissingPolicy,
  InvalidWork, DuplicateAdmission, BusyBoundary, NoPendingPolicy. Match exact
  variant names in C2.1 tests; Error/Debug/Eq and unchanged-state assertions.

The helper never changes FlowRuntime or draws randomness. It is execution
admission support, not a disconnected simulator: boundary and owner checks read
actual WorkSpec/WorkProgress. Actual service/transit execution is tested against
Flow under C2.2/C2.3. Initial scope has one global quiescent policy replacement;
per-scope switching may be added with a separately reviewed contract.

## Paired execution and providers

Track21 reuses CalibrationSeedMap v1, the one advancing service stream owner and
opaque owned snapshot. No DES->calibration dependency: preserve default Rust1.76
versus optional calibration Rust1.88. A calibration-only dev dependency on DES
may support integrated fixtures without changing engine dependency floors.

Service duration is intrinsic work in declared ticks. Fixed or empirical
weighted discrete inputs must reject zero durations, invalid/overflow weights,
missing strata and mappings that include queue/transit intervals. Sample once
at task admission; Suspend resumes remaining work, Restart reuses the original
sampled duration. Never resample service on policy change/resume/restart.

Macro schedules no transit event and spends no transit draw. Zero-transit Micro
also schedules no transit event and spends no transit draw; both use identical
service seed identity and produce identical Flow task timing/outcome and service
draw position. A nonzero transit leg may draw only the Transit purpose stream;
Behavior has its own stream. Snapshot restoration preserves all owned purpose
streams, admission decisions, work progress and pending configuration in memory.
No portable persistence claim follows: Track22 codec/compatibility is an explicit
remaining gate, not a stringly-typed serialization shortcut.

## Test-first evidence and delivery gates

C2.1.mode tests exhaust precedence, absence, malformed subsystem/version,
generational entity isolation, duplicate/unknown admission, staged update and
Pending/Active/Suspended boundary rejection using actual Flow work. Terminal
boundary applies only to future admissions; compare original work/context bytes
and immutable decisions before/after. C2.1.paired then tests common-random-number
identity and actual Flow outcomes, resume/restart and purpose separation.

Tests are written before production helpers. Retain expected initial compile/test
failure as red-phase evidence; no failing/ignored test or missing runtime is
converted into acceptance. Join C2.1 only after every leaf is exercised by actual
runtime, independent review and exact-head native owner CI. C2.2 provider/admission
and C2.3 routes remain separate original milestones. Do not mark full C2 accepted
from resolver fixtures. Q5.2/C1.4 thresholds and Track49 work stay untouched.

Compatibility/objection: experimental private module adds no public export,
changes no scheduler/RNG algorithm and requires no external dependency. Reviewer
objection that a caller might omit active work is addressed by adapter-owned
bindings and checks of all admitted actual Flow work. Unknown/despawned bindings
block updates until a future explicit lifecycle cleanup contract is reviewed.

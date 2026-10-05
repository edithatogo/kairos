# C2.0 provider and transit interface proposal v1

Status: proposal awaiting independent owner review. This document grants no
runtime acceptance, public export, checkpoint portability or clinical validity.
Base source: 65858adfff7976f90802f9cda20aa34b6fd64d90. Track21 owns the empirical
provider; Track03 owns actual Flow admission and transit; Track01 owns seed-map
v1; Track22 owns runner/portable restoration. C4 metrics remain separate.

## Provider representation and sampling

Implement a private calibration module `work_duration.rs`, using existing
`CalibrationStream` and `SimDuration`; no DES-to-calibration dependency.
`IntrinsicDurationDistribution::fixed(ticks: u64)` and
`::weighted_ticks(support: Vec<(u64, u64)>)` return checked distributions.
Preserve supplied support order as the versioned inverse-CDF order. Reject empty
support, zero duration, zero weight, duplicate duration, checked total-weight
overflow and unsupported versions. Equal durations must be aggregated explicitly
by the fitting stage. Inputs represent intrinsic work only: fitted queue, wait,
boarding or travel intervals cannot masquerade as intrinsic service observations.
The caller supplies documented training provenance and strata mappings; unknown
strata fail rather than use an invented default.

`sample(&self, stream: &mut CalibrationStream) -> Result<SampledWorkDuration,
WorkDurationError>` returns checked ticks and before/after draw positions.
Fixed inputs consume zero draws. Weighted inputs use u64 rejection sampling:
threshold = total.wrapping_neg() % total; reject values below threshold and use
accepted value % total as a zero-based cumulative-weight index. Never use float
scaling or biased modulo without rejection. Rejection draws count as actual
transitions. Sampling errors cannot be reported as successful admissions.
Service streams are created only with Service purpose and retained once per
logical study/schedule/replication/case/task identity. Candidate and execution
mode never enter that identity. No stream is recreated at position zero on retry.

## Actual admission bridge

Keep the initial model bridge private and outside the core scheduler. A sampled
preparation record owns original intrinsic duration, frozen logical keys and
service continuation. It is consumed when constructing actual Flow work through
`FlowRuntime::create_work` or `create_restartable_work`, then bound through the
existing `FidelityAdapter::admit`. Preparation and admission are separate typed
states: creation failure retains the preparation record for retry without a new
sample; duplicate admission rejects. Do not claim a multi-object atomic commit
before implementation proves preflight/allocation/rollback behavior.

Mode is resolved before transit is requested and frozen when actual work is
admitted. Runtime identity prevents foreign WorkId collisions. The bridge records
actual WorkId and observed `WorkSpec::original_duration`; tests must compare it
to the sampled record, not a parallel bookkeeping counter. Timed claims use real
`FlowRuntime::acquire(...).timed_work(...)` and actual completion dispatches.
Suspend preserves remaining duration; Restart uses the originally sampled
intrinsic duration and context template, with no additional Service draw.

## Transit model and observation seam

C2.3 supplies an immutable versioned graph: stable node/edge IDs, nonnegative
integer millimetres, explicit movement mode and positive millimetres per second,
explicit simulation ticks per second. All conversion uses checked integer
arithmetic and ceiling division. Zero-distance travel consumes no Transit draw
and schedules no transit event. Missing nodes/unreachable routes, zero speed,
overflow and incompatible graph versions fail explicitly.

Shortest routes tie by total distance then the full stable edge-ID sequence;
coordinate plotting order and HashMap iteration must never select a route.
A route receipt carries graph version/hash, origin/destination/purpose, edge IDs,
length and duration. Macro bypasses transit entirely. Nonzero Micro movement
schedules real registered domain events through Flow, stores edge/progress state,
and issues service acquisition only after actual arrival dispatch. Travel is a
separate interval from useful work and queue wait. Interrupted movement retains
position, remaining ticks and route identity; resume cannot duplicate arrival or
resource claims. Staff dispatch policies (urgency/FIFO/zone/skill) remain model
adapters, with separately reserved cleaning/turnaround resources.

## Continuation boundary

Opaque provider snapshots preserve actual purpose-stream state and sampled
records. They do not snapshot FlowRuntime. Current Flow has no complete runtime
checkpoint codec; retaining existing actual work in memory is not full restore.
Track22 must implement/review the runtime bridge preserving scheduler, work,
resource ownership, pending policy, route progress and purposes before full
checkpoint acceptance. Restored records cannot bind a fresh runtime using only
colliding WorkIds. Unknown versions and graph/policy/stream mismatches fail closed.

## Test-first acceptance definitions

C2.0 red fixtures target missing real provider/transit bridge APIs. Compile-red
proves absence only; C2.1 acceptance requires final behavioral execution.

- Weighted bounds, overflow, duplicate support and unknown strata reject; fixed
  inputs consume zero draws; pinned golden vectors distinguish inverse-CDF order.
- Actual paired Macro and zero-transit Micro use the same service seed identity,
  sampled duration, WorkSpec, completion ticks/outcomes and draw positions. Both
  have no transit event, route state or Transit draw.
- Nonzero Micro leaves Service stream unchanged relative to Macro; Transit and
  Behavior have separate ownership. Flow dispatch receipts expose added travel.
- Real Suspend and Restart interruption tests conserve original sample/context,
  remaining/useful/cumulative busy accounting and exact completed draws.
- Failed preparation/admission retry does not resample. Foreign runtime, unknown
  work, stale logical identity and active/suspended mode replacement fail closed.
- Interrupted route arrival is emitted once; queue+travel+useful-work reconcile
  in integer ticks; deterministic route ties hold under permuted graph input.
- Provider-only continuation reproduces the next actual draw; full runner restore
  remains a separately required gate until real runtime checkpoint tests pass.

Default DES Rust1.76 remains unchanged. Calibration Rust1.88 and canonical1.99
must pass locked tests; calibration-only DES dev dependency may enable real
integration fixtures. Public API adoption needs ADR, conformance, compatibility
and objection review; this proposal does not export experimental helpers.

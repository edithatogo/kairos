# C2.0 intrinsic work provider v1 — frozen sampling contract

Scope: Track21 pure provider test/implementation interface, independently reviewed
by d35_gate_review and resolved by the coordinator. This is one C2.0.interfaces
instance. Admission, transit, paired execution and full C2.0 remain open. No
public exports, empirical fitted model, runtime retry or checkpoint acceptance.
Base: 8ac079e; seed framing/generator remain calibration-seed-map.v1/stream.v1.

## Exact private production interfaces

`work_duration.rs` is private production calibration code. Types and methods
below are `pub(crate)` so sibling adapters can consume them. No DES dependency
is needed for this pure module. Public API review and production Flow bridge
are separate required instances; a test import cannot replace that bridge.

```rust
pub(crate) const INTRINSIC_WORK_PROVIDER_VERSION_V1: u32 = 1;
pub(crate) struct IntrinsicDurationDistribution { /* private */ }
pub(crate) struct IntrinsicWorkProvider { /* private */ }
pub(crate) struct SampledWorkDuration { /* private duration/key/draw bounds */ }
pub(crate) enum WorkDurationError {
    UnsupportedProviderVersion(u32), EmptySupport, ZeroDuration, ZeroWeight,
    DuplicateDuration, WeightOverflow, InvalidStratum, DuplicateStratum,
    MissingStratum, WrongPurpose, IdentityMismatch, Seed(CalibrationSeedError),
}
impl IntrinsicDurationDistribution {
    pub(crate) fn fixed(ticks: u128) -> Result<Self, WorkDurationError>;
    pub(crate) fn weighted_ticks(support: Vec<(u128, u64)>)
        -> Result<Self, WorkDurationError>;
    pub(crate) fn sample(&self, stream: &mut CalibrationStream,
                         expected: &CalibrationStreamKey)
        -> Result<SampledWorkDuration, WorkDurationError>;
}
impl IntrinsicWorkProvider {
    pub(crate) fn new(version: u32,
        strata: Vec<(String, IntrinsicDurationDistribution)>)
        -> Result<Self, WorkDurationError>;
    pub(crate) fn sample(&self, stratum: &str, stream: &mut CalibrationStream,
                         expected: &CalibrationStreamKey)
        -> Result<SampledWorkDuration, WorkDurationError>;
}
impl SampledWorkDuration {
    pub(crate) fn duration(&self) -> SimDuration;
    pub(crate) fn draw_before(&self) -> u64;
    pub(crate) fn draw_after(&self) -> u64;
}
```

Distributions are v1 payloads by type. Provider `new` owns version validation;
unsupported versions reject before validating strata. Distribution constructors
have no competing version field. Weight support preserves supplied order, rejects
empty/zero duration/zero weight/duplicate duration and checked weight-sum overflow.
Validation walks rows in order; duration then weight then duplicate then sum.
Fixed duration must be positive. u128 ticks map directly to SimDuration and do
not narrow to u64. No float probability, unit inference or default service time.

Provider validates all stratum IDs before duplicate checks. IDs follow seed-map
canonical rules: nonempty, at most1024 UTF8 bytes, no controls or boundary
whitespace; exact byte identity. Unknown valid lookup -> MissingStratum, malformed
lookup -> InvalidStratum. Empty map -> EmptySupport. Duplicate strata reject;
no silent overwrite. Error and sample Debug output must not expose raw logical
identity. Errors implement Debug/Eq/PartialEq; Seed retains the typed seed error.

## Expected logical identity and one advancing stream owner

Add crate-private opaque `CalibrationStreamKey` with private SeedIdentity fields
and Eq/PartialEq (no public constructor, serialization, raw identity Debug).
`CalibrationStream::key(&self)` returns this key; `purpose(&self)` returns purpose.
`CalibrationSeedMap::key_for(&mut self, schedule: &str, replication: u64,
case: &str, task: &str, purpose: SeedPurpose) -> Result<CalibrationStreamKey,
CalibrationSeedError>` validates/registers the same full identity as stream_for
without constructing a second advancing stream. It is idempotent for one identity
and preserves collision rejection. Expected keys originate from this independently
selected logical task, not from blindly trusting an input stream's key.

`CalibrationStreamSnapshot::restore_for(self, expected: &CalibrationStreamKey)
-> Result<CalibrationStream, CalibrationSeedError>` compares full expected identity
before restoring; mismatch returns InvalidSnapshot. Existing public restore keeps
its behavior. Full identity includes version/root/study/schedule/replication/case/
task/purpose; derived-seed equality alone is insufficient. Service sampling checks
WrongPurpose first, then IdentityMismatch, including fixed distributions.

## Deterministic sampling and failure ownership

Fixed sampling returns its duration with unchanged draw bounds. Weighted sampling:
`threshold = total.wrapping_neg() % total`; draw u64 until value >= threshold,
then use value % total as zero-based cumulative-weight index. Rejected transitions
count. Candidate/mode never enter service seed identity. Return before/after
actual CalibrationStream positions and the immutable original sampled duration.

Sample on a temporary stream restored from the owner's opaque snapshot. Commit
the temporary stream back only on success. Errors, including overflow after a
rejection, leave the original next draw and position unchanged. Successful sample
advances the caller's stream; SampledWorkDuration does NOT own continuation.
The later bridge must transfer sample+continuation into its owned preparation
state and handle partial Flow creation. Provider API alone guarantees no retry,
policy freeze, interruption accounting or complete checkpoint restoration.

## Test-first oracles

- Canonical golden identity from c2-seed-map-v1.md, ordered support
  [(10,3),(20,2),(30,5)]: total10, threshold6; accepted first three values have
  residues7/2/4 -> durations30/10/20 with cumulative positions1/2/3.
- Same seed with reversed support order gives its explicitly recalculated mapping;
  do not sort support inside the sampler. Equal support/order yields exact outputs.
- Support boundary index belongs to the next cumulative bucket; use independently
  computed real RNG fixtures, not a fake provider or disconnected simulator.
- Rejection fixture uses real stream transitions and a large total weight; count
  rejected draws. Overflow fixture in seed_map unit tests may inject private
  draw position at MAX; production has no arbitrary-state constructor.
- Fixed supports u128::MAX, validates purpose/identity, and consumes zero draws.
- Every constructor/lookup error and unknown version has exact typed failure;
  invalid/missing stratum, wrong purpose and wrong expected task leave next draw
  equal to a saved same-stream continuation. Duplicate strata never overwrite.
- restore_for rejects another case/task/schedule/root/replication/purpose and
  preserves next-draw equivalence for the matching key. Public seed golden stays
  unchanged. Provider-only restore is not restoration of actual Flow state.

Default DES1.76 remains untouched; optional calibration1.88 and canonical1.99
locked tests are required when implementation exists. Red compile evidence for
absent work_duration is preparation only. This contract introduces no fitted EHR
claims or policy, and queue/transit intervals remain excluded intrinsic inputs.

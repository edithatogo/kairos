# C2.0 submitted Service observation v1 — additive interface

Status: proposed for bounded independent review; no runtime acceptance.
Owners: Track21 stream/bridge, Track03 actual Flow, coordinator test definition.
Extends `c20-admission-bridge-v1.md` without changing its existing signatures.

## Missing observability

Bound exposes Service draw position, but submit/finish consume Bound. The
required actual Suspend/Restart oracle needs the owned stream position after
submission and actual engine execution. Asserting only the pre-submit position
cannot prove absence of later resampling. No duplicate stream, fake counter or
relaxed oracle is an acceptable substitute.

## Frozen additive signature

```rust
impl<T:Clone,C:'static> SubmittedIntrinsicWork<T,C> {
    pub(crate) fn service_draw_position(&self)->u64;
}
```

The read-only getter reports the retained actual advancing Service stream's
position. It performs no draw, reconstruction, snapshot restore or mutation.
Submitted keeps that single owned continuation available while callers execute
the actual Flow runtime; actual task creation/retry/preemption never transfers
it to a second advancing owner or resamples intrinsic duration.

Tests compare the actual position before submission and after accepted arrival,
Suspend/resume and Restart completion, alongside actual WorkSpec/progress,
resource requests and context/template. Provider snapshot proof remains distinct
from complete Flow checkpoint restoration. No public export or codec is added.

## Native test-first wiring

Absent production module files may be path-declared at the native test crate
root so the Rust parser reports their precise missing-file error before name
resolution. Expected red requires real compiler/file evidence, not a predicted
missing-dependency failure. The current manifests remain unchanged during C2.0.
Future green must enable the implemented production `flow` feature and use actual
DES/ABM exports/dependencies. Test-only imports do not qualify production wiring.
No disposable alternate engine, dependency patch, generated fake bridge or
changed Cargo lock is permitted to satisfy runtime acceptance.

All active paired workers are stopped at this contract barrier. Coordinator
rebinds source/base/input hashes and bounded context after review before dispatch.
Original Macro/Zero no-transit and preemption requirements remain unchanged.

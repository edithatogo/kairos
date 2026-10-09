# ADR-0013: closed-scenario C2 replay journal

**Status:** bounded experimental prototype; no C2 phase, C5 recovery, release,
clinical, hosted, or acceptance gate is closed.

## Decision

Implement a private replay journal for one sealed synthetic scenario using the
actual calibration provider, seed map, annotated Micro route, Flow bridge, and
TransitContext. The fixture uses one configured five-tick route, one actual
Service stream draw, and fixed pause/resume controls. Its only supported
checkpoint frontier is after start-at-zero and pause-at-one have been dispatched
and reconciled, with resume-at-three already queued and the replaced progress
event-at-five still pending. The replay suffix must classify that old event as
stale once, reach the real arrival at tick seven once, and preserve the five
movement ticks and two paused ticks.

The artifact stores the complete allowlisted setup, seed identity, provider
configuration, graph/profile/OD/tick rate, route metadata, fidelity policy,
synthetic owner/work/resource/request identities and snapshots, registered
handler inventory, operation schedule, supported frontier values,
compatibility fingerprints, and a canonical prefix digest. Ticks and other
full-width integers use canonical decimal strings. The fixture/schema/handler inventory,
scenario config and handler inventory are compiled constants. The source/build
compatibility fingerprint is the SHA-256 of the exact current test executable;
the child must run that same executable and compare its own binary hash before
replay. This binds source, compiler output, and linked dependencies without a
cross-build or cross-toolchain promise. Every artifact field is compared with
these independent expectations before any artifact value can construct the
Flow, seed/provider/route, handler registration, or schedule. Artifact fields
never select arbitrary code or authorize a different recipe. The reader checks file
metadata first, rejects symlinks and non-regular files, rejects files over one
MiB, then reads at most limit plus one byte and verifies the opened file still
matches the prechecked device/inode. It rejects noncanonical JSON (including duplicate members),
unknown/missing fields, unsupported schema/config/frontier, bad integrity, and
truncated input. Negative controls cover duplicate members, noncanonical
integer strings, unknown/missing members, truncation, oversize input, corrupt
digests, and wrong-but-rehashed scenario config, frontier, handler inventory,
and executable fingerprint. Each rehashed mutation must fail independent
expected-value comparison. Integrity detects corruption; it is not
authentication.

Restoration constructs a fresh Flow runtime and replays the complete prefix in
the original order, including setup, Service draw, event creation, dispatches,
and bridge observations. Before suffix execution it validates the stored route receipt against the
actual immutable carrier, includes that stored receipt SHA in the frontier,
and compares live owner/work/request/resource state, fidelity decision, IDs,
event queue counts, context phase/progress and expected pending event, stream
draw position and a snapshot-based next-draw probe, and the canonical prefix
digest with independently specified expectations and the artifact. The start
dispatch must admit exactly the pending progress EventId; the paused carrier
must still expect that same event at route-derived tick five. The prefix digest covers canonical setup and
operation prefix, dispatched event IDs/ticks, lifecycle/control observations,
frontier state and scheduler counters. The child entry point is one exact named
test helper with a fixed invocation; the artifact cannot choose the helper. The reproduced digest must match both
the artifact digest and a compiled expected prefix digest before suffix work. It
never serializes or compares `FlowRuntimeIdentity`, runtime debug output, or an
opaque in-memory continuation.

A fresh child process loads the published artifact and performs the replay via
one exact named test-harness helper invocation and an allowlisted helper ID;
unknown helper/handler IDs reject before replay. Its exact suffix trace checks resume at tick three, the original progress ID
stale exactly once at tick five, one actual route arrival at tick seven, then
the same linked service request's completion at tick twenty-seven with twenty
useful ticks, zero remaining, one Completed lifecycle record, and the resource
released. Terminal IDs/outcomes, resource/work/request observations, and the
next Service draw are compared with an uninterrupted baseline. Publication
uses a same-directory, create-new temporary file, file sync, and atomic
no-overwrite hard-link to the target followed by directory sync; an existing
target is left byte-for-byte unchanged.

## Compatibility and scope

The journal is private, versioned, and limited to the exact synthetic recipe and
supported pause frontier. An existing-destination negative control seeds a
sentinel file, attempts publication, and proves the sentinel bytes are
unchanged. It supports no user callbacks, handler aliases,
custom operation inventory, arbitrary scenario parameters, runtime heap
serialization, cross-version/platform promise, or C5 candidate/worker ranking
recovery. Recovery cost is one complete deterministic prefix replay; this
trades execution time for a small persisted state and avoids private heap
serialization. Any source/config/schema drift fails closed. The
`FlowRuntimeIdentity` remains process-local ownership state and is never
serialized.

## Verification

Verified with Rust 1.99.0 from the Rustup toolchain at
`/Users/doughnut/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin` (including
`cargo`, `rustc`, `rustdoc`, and Clippy), with the isolated target cache under
`.artifacts/c2-checkpoint-replay/target-rustup`:

- `cargo test -p kairo-ecs-calibration --features flow,kairo-ecs-abm/test-support c2_checkpoint_journal::tests -- --nocapture`: 4 passed, 0 failed, 105 filtered. The child loaded the retained artifact itself; artifact SHA-256 `7e468b7f6df58bd2dc3079e9491b177ef895849459cec9ef335d98db3c975639`. Log SHA-256 `429da19601456b3ee0a7c4dbbb4506a5ceba32a5814d77cc908c50866601d3f5`.
- `cargo test -p kairo-ecs-calibration --features flow,kairo-ecs-abm/test-support`: library 107 passed, 0 failed, 2 ignored; all integration targets passed. Log SHA-256 `0ee876be3ee660d737ff367766a72699dba34e7398d91d4aba79d3ccfc2f5b30`.
- `cargo clippy -p kairo-ecs-calibration --features flow,kairo-ecs-abm/test-support --all-targets -- -D warnings`: passed. Log SHA-256 `0bca1300d6dd5b2bc3e33176b9e5bb81185f5bdfcba9b67ee1935755881b6011`.
- Rustfmt check of the new journal module and `git diff --check`: passed.

All logs and artifacts are retained under `.artifacts/c2-checkpoint-replay`.
Passing this closed fixture does not establish portable general recovery, C5
recovery, hosted checks, or phase acceptance.

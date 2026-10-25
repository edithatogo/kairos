# Track 106: Scheduler Preview API Compatibility

Status: In Progress. Candidate base: `5692e6a9ee866131262d60103a84e2544dc51e4a`.

## Objective

Restore the additive Rust API `ScheduledEventPreview` and
`Scheduler::peek_next(&mut self)` required by CareOps R10.3. Preserve the
accepted scheduler ordering and dispatch behavior; a preview observes the next
live event without dispatching it.

## Inputs

- Exact candidate base `5692e6a9ee866131262d60103a84e2544dc51e4a`.
- Track 01 core contract and scheduler ownership, Track 12 fixture contract,
  Track 25 API compatibility rules, and the CareOps R10.3 consumer contract.
- Previously retained API shape at Kairos `44261eb5cf8275f132f63a3396903d116d701dbe`.

## Outputs

- Public `ScheduledEventPreview` projection with `id`, `at`, `priority`,
  `sequence`, `entity`, and `kind` fields.
- `Scheduler::peek_next(&mut self) -> Option<ScheduledEventPreview>` which
  prunes dead heap heads as necessary and does not dispatch or change virtual
  time, scheduler counters, or live pending membership.
- A deterministic shared preview fixture, manifest entry, and Rust consumer
  test covering empty, ordering, cancellation, repeat observation, and the
  following dispatch.
- API rustdoc, a compatibility ADR, exact CareOps downstream validation, and
  owner review receipts.

## Acceptance criteria

1. Empty scheduler returns `None`.
2. Preview identifies the next live event using existing `(at, priority,
   sequence)` order and returns a complete copied projection.
3. Cancelled events are never previewed; pruning may remove dead heap entries
   but does not change live pending membership or statistics.
4. Repeated preview calls do not dispatch, advance time, alter counters, or
   change pending live events; `step()` then dispatches the previewed event.
5. Shared fixture validation and focused core tests pass.
6. The API Governance, core scheduler, and conformance owners review the exact
   candidate. The user as CareOps consumer owner records the R10.3 validation.
7. The CareOps parent pin remains unchanged until all prior gates pass and the
   accepted Kairos commit is available.

## Owned paths

- `conductor/tracks/106-scheduler-preview-compatibility/`
- `crates/kairo-ecs-core/src/lib.rs` (additive preview surface only)
- `crates/kairo-ecs-core/tests/track106_scheduler_preview.rs`
- `conformance/fixtures/track106-scheduler-preview.json`
- `conformance/fixtures/manifest.json` (one fixture registration only)

The user-approved bounded track authorizes these exact additions; existing
Track 01/12/25 owners retain review authority over their respective surfaces.

## Blocked paths

- Queue ordering, scheduling, cancellation, dispatch, time, or stats semantics.
- Track 25 policy/inventory files, Track 01/12 owned source beyond the exact
  listed additions, FFI, bindings, Arrow/Parquet, and CareOps parent pin files.
- Publishing, push, release, or claims of downstream acceptance without their
  required authorization and exact evidence.

## Dependencies and sequencing

Tracks 01, 12, and 25 provide the core, conformance, and compatibility
contracts. Their owners must review before acceptance. CareOps R10.3 is the
downstream consumer gate. Implementation begins with failing fixture-driven
tests, then restores the additive API, documents it, and runs exact local and
downstream checks.

## Parallel-safe tracks

Read-only CareOps review is parallel-safe. Writes to the registered core,
conformance, registry, or Track 106 paths are serialized under this track's
single-maintainer lease.

## Quality gates

See [test matrix](test-matrix.md). Track phase review, Conductor validators,
focused Cargo tests, shared fixture validation, and the exact CareOps R10.3
build and reference tests are required. Local evidence does not substitute for
owner review or upstream acceptance.

## Release implications

The API additions are compatible with the retained API shape and do not alter
the Rust type/API root or scheduler behavior. Track 25 must confirm that
classification. No release, stable compatibility, or full CareOps acceptance
claim is made by this track alone.

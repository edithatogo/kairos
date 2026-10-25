# ADR 106.1: Restore additive scheduler preview compatibility

- Status: Proposed; implementation approved in the bounded Track 106 scope.
- Decision date: 2026-10-25.
- Review: Track 25 API Governance and Track 01 core scheduler owner review are
  pending against the exact candidate commit.

## Context

CareOps R10.3 uses `ScheduledEventPreview` and `Scheduler::peek_next`, present
at the retained CareOps pin but absent from the candidate Kairos base
`5692e6a9ee866131262d60103a84e2544dc51e4a`. CareOps therefore cannot compile
against the candidate without restoring that API surface.

## Decision

Restore the copied preview value type and a mutable `peek_next` observer that
prunes cancelled heap heads and copies the existing next live queue entry. Keep
the current `(time, priority, sequence)` ordering and dispatch implementation
unchanged. Repeated preview is not a dispatch and must not change virtual time,
statistics, or live pending membership.

## Consequences

- The Rust core API is additive and preserves the retained consumer signature.
- The method takes `&mut self` because pruning stale heap entries can mutate the
  internal queue; it does not promise a stable result after subsequent schedule
  or cancel calls.
- A deterministic shared fixture and focused core test pin the observable
  contract.
- API compatibility and conformance owner review are required before acceptance.
- CareOps pin changes remain a separate gated action after the exact consumer
  validation passes.

# ADR: refuse unproved network reclamation floors

Status: source candidate; independent implementation review and hosted checks pending.

## Context

CAP-48.1 requires complete distributed GVT under delayed/duplicated traffic,
worker failure and restart. The current network reductions sample inboxes before
all sends complete. They can return a positive scalar minimum without complete
channel/retirement/recovery accounting. Peer count does not close this gap.

## Decision

Return ZERO through the scheduler floor interface for every live Tonic, custom
TCP and MPI-launcher TCP backend, even with no configured peers. Preserve the
existing algorithms as explicitly diagnostic `reduce_reported_minimum` methods;
move numeric arithmetic tests to that boundary and add responsive-network floor
regressions. Clarify the trait's conservative-floor contract. Keep closed local
emulators, actual MPI preview, wire encoding, native owned admission/retirement
and fossil-collection proof constructors unchanged.

## Public API/compatibility and objection response

This is an additive diagnostic API and a deliberate restriction of previously
unproved network progress. Existing scheduler/LP clocks do not regress, but no
new positive distributed GVT is claimed. Removing the restriction requires the
complete cut protocol; a scalar minimum must not be used as a fallback. Numeric
tests remain, not waived or renamed as distributed acceptance. New safety tests
verify the scheduler's separate refusal path. No ABI/schema/fixture changes.

Independent design review found that a proposed no-peer exception was unsafe
because listeners accept remote sources. That exception was removed. A proposed
two-phase sample was rejected as a replacement for authenticated, durable cuts;
it would still require coordinated send/model quiescence and failure fencing.
Full design and reproduction are in [the safety note](../../../docs/distributed/unproven-cut-safety.md).

## Acceptance boundary

CAP-48.1 remains unchecked and Track 48 In Progress. Source safety, local tests,
hosted integration and two-host runtime acceptance remain separate. No main,
parent pin, native service, authentication or publication state is promoted.

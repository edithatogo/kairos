# ADR 0028 Opt-in scheduler reservation and hwloc discovery

## Status

Accepted for the bounded Rust API in Track 50. The track retains its live
multi-node placement and performance gate.

## Context and decision

The production scheduler owns a heap and a pending-event set. Callers with
a measured queue peak can use `Scheduler::with_capacity(usize)` to allocate
both up front. The pending set reserves additional space for removal and
reinsertion. The initial heap reservation includes cancelled entries that
still await pruning. Storage grows when it needs to and follows the normal
cancellation compaction policy.

The optional `numa` feature exposes `HwlocTopology::discover(&Path)` for
CPU groups, cache sharing and memory domains. Callers supply the trusted
absolute path to an installed hwloc 2.x tool. The crate invokes the XML
export directly with a clean child environment and parses the existing
locked quick-xml dependency. It reports launch and format failures as I/O
errors. The existing `NumaTopology` supplies its kernel-discovery and
explicit synthetic profiles.

## Compatibility and ownership

The new constructors are additive to `kairo-ecs-core`. Existing callers
can continue to use their current constructors. Event identities, tie
ordering, cancellation, scheduler counters and checkpoints keep their
existing meanings. The C ABI and Arrow schema retain their published
contracts. The host owns executable selection and placement. A scheduler
owns its queue independently, and the compiled C and PyArrow consumers
exercise the separate Arrow C Data owner-transfer contract.

## Red-team responses

- An excessive estimate can consume memory early. Reservation is explicit.
  Use an application-owned queue estimate, and account for its extra
  pending-set allocation in the memory budget. Construction follows Rust
  collection allocation limits.
- A tool path or environment override can supply a fabricated topology.
  The caller selects a trusted absolute executable path, the child uses
  a clean environment and the parser checks the format and typed fields.
  The native Linux result is compared with independent sysfs records.
- A topology record cannot establish memory placement. The allocator
  samples qualify warmed allocations and lifecycle on the measured hosts.
  CAP-50.5 requires live page-placement readback and matched timings across
  two memory nodes.

## Conformance

The versioned scheduler fixture exercises growth, cancellation, equal-time
priority and insertion ordering. The Node runner independently checks the
expected dispatch kinds. The Rust test executes default, reserved and
checkpoint-restored schedulers and checks their counters.
The hwloc XML fixture exercises a multiword CPU bitmap, shared cache
membership and memory-domain edge cases. Native discovery and allocator
captures identify their exact hosts and build inputs.
[Scheduler fixture](../../conformance/fixtures/track50_scheduler_reservation.json) ·
[Topology fixture](../../conformance/fixtures/track50-hwloc-v2.xml) ·
[Native qualification](../../conductor/tracks/50-numa-topology-affinity-memory-lifecycle/evidence/native-qualification/README.md).

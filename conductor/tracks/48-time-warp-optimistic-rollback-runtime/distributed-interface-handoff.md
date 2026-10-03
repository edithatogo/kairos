# Proposed Track 48/49 interface handoff

Status: draft for review. This document grants no Track 49 production dispatch,
changes no dependency or acceptance gate, and records no execution pass.
The approved local implementation contract is [optimistic-runtime-contract](../../../docs/pdes/optimistic-runtime-contract.md).

Track 49 depends on Track 48, while Track 48 Done requires live distributed
rollback evidence from their integration. The proposed resolution is an explicit
phase handoff: accept Track 48's reviewed local runtime and transport interface,
merge its implementation-phase PR with status In Progress, authorize Track 49
against that accepted interface, then return to Track 48 for distributed
acceptance. A local merge alone does not authorize that scheduling change.

## Proposed handoff gate

Before requesting the decision, provide the exact implementation commit and PR,
independent replay/cancellation/state reviews, actual pinned native and hosted
checks, raw sparse/dense benchmark evidence, and an accepted wire contract.
Keep unavailable or failed gates explicit. Future npm exception authority cannot
come from EXC-193, which applies only to PR #193.

## Wire requirements to review with distributed-agent

Preserve RemoteEvent source LP, destination LP, tick and event_payload bytes. Evidence hashes remain separate metadata. Carry a
separate full logical ordering identity and an exact delivery incarnation.
Identity must retain full parent ordering keys and vector ordinals; it cannot be
replaced by arrival order or a lossy hash. Bound causal ancestry to 128 and reject
malformed/oversize input before mutation. A future decoder requires its own
bounded validation; the local opaque Rust identity is not a validated wire codec.

Every positive and matching anti identifies the actual emitting source LP,
opaque logical ID and incarnation. Replay preserves logical ID and produces a
fresh incarnation. Namespacing must use the actual emitter rather than ancestral
root source. Duplicate delivery is idempotent or rejected consistently; conflicting
metadata fails unchanged. Old antis cannot cancel replacement incarnations.
Transport retains in-flight positives and antis in GVT accounting and cannot
report delivery before its agreed receipt boundary. Explicitly define retry,
acknowledgement, shutdown and failure behavior before implementation.

## Required integrated evidence

| Scenario | Required observation |
| --- | --- |
| Cross-process/rank straggler | Restore complete model and RNG state; replay matches independent sequential state and committed logical trace. |
| Invalidated downstream sends | Retract recorded outputs across participants; replays use new incarnations. |
| Replacement before old anti | Old anti cancels only the old incarnation; replacement survives. |
| Anti before positive and duplicate retries | Exact tombstone/duplicate semantics without double application. |
| GVT advancement | Include in-flight and queued work across participants; collect strictly before floor and keep equality reversible. |
| Failure and recovery | Classify failures and preserve cancellation, pending delivery and generation metadata. |

Record exact source, commands, toolchains, topology, MPI/gRPC implementation,
seeds/input hashes, raw logs, exit statuses, artifact hashes and independent
review. Track 49 additionally requires real 2/4-rank MPI and two-process socket
runs under its own plan. Local in-process runs and manifest validation do not
satisfy these requirements. Track 48 remains In Progress until the live
integration evidence is accepted; publication and broader HPC claims retain
their separate gates.

## Ownership

Track 48 retains runtime semantics and cancellation/GVT integration. Track 49
retains wire codecs, MPI/gRPC delivery and distributed failure behavior. Core ECS,
debug, Arrow writers and launch infrastructure remain under their existing
owners. Any interface changes return to both owners for review. This proposal
needs the coordinator/user scheduling decision before production dispatch.

# Proposed Flow continuation context and checkpoint boundary

Status: proposal for owner and track review; not an approved API or implementation.
Date: 2026-09-29
Scope: Q0.1 codec ownership boundary for Flow continuation state.

## Decision

Keep queue/work continuation state as one typed, owned, in-memory context within the proposed shared `FlowRuntime`. That context owns the work and interruption records needed to continue execution, together with deterministic registration identity and explicit lifecycle transitions. It is runtime state, not a portable checkpoint format. Do not add `serde`, a codec registry, public serialization APIs, or portable checkpoint claims in this work.

The inspected Kairos state surface does not identify a registered context codec or a portable serializer for complete simulation state. This is a bounded finding about the inspected sources, not proof that no such work exists elsewhere. Track 22 is the owner for experiment checkpoint and resume semantics and should reconcile any additional implementation or design before a format is specified.

## Current boundary

`kairo-ecs-state::WorldSnapshot` is a deterministic view of live entity IDs. Its `EntitySnapshot` contains only an `EntityId`; it does not capture component values, scheduler events and ordering counters, random-number generator state, Flow queues and allocations, active work, interruption context, or adapter registrations. It is therefore not a resumable simulation checkpoint and must not be presented as one.

The `checkpoint` and `resume` CLI commands described by the Track 22 handoff are scaffold surfaces: checkpoint writes a scaffold manifest, while resume validates a path and writes a scaffold request. They do not establish complete state capture, restore behavior, or production resumability.

Until upstream owners define a portable format, Flow continuation context remains owned by its live `FlowRuntime` and is usable only during that runtime's lifetime. Callers cannot register arbitrary serialized context or treat an entity-only `WorldSnapshot` as a substitute.

## Future codec ownership

Track 22 owns the experiment-runner checkpoint/resume contract and should lead a later, explicit design for versioned portable serialization. That design must coordinate with Track 01 for core/state snapshot boundaries, Track 03 for behavior and context registration, Track 04 for RNG/seed state, and Track 22 runner owners for experiment identity and resume semantics, as relevant. These owners must agree which state is required to reproduce continuation before a codec or registry API is proposed.

Do not introduce a second snapshot standard inside the queue/Flow work. The format, codec registration API, supported-version policy, and migration behavior remain open for those owning tracks. Any future design should inventory existing upstream codecs first and reuse or extend the accepted owner contract where appropriate.

## Determinism and compatibility gates

Before a future codec is accepted, its contract and tests should demonstrate:

- complete capture and restore of the state required for continuation, including scheduler ordering state, entities/components, Flow queue/work/interruption state, behavior registrations, and random state where those are part of the supported run;
- deterministic encoding for equivalent states and deterministic restoration independent of incidental map or host iteration order;
- continuation equivalence between an uninterrupted run and a save/restore run at the same event boundary, including event trace and terminal metrics under fixed seeds;
- explicit versioning, supported-version and migration policy, plus rejection tests for unknown versions, malformed/truncated data, missing required state, and incompatible registrations;
- preservation of existing snapshot meaning and legacy API behavior, with compatibility classification and release review by the owning Track 25 process before any public promise.

These are gates for future codec work, not claims that a codec or resumable checkpoint currently exists. In-memory Flow execution must continue to preserve the established scheduler ordering and RNG behavior without relying on serialization.

## Open owner review

1. Track 22: confirm checkpoint/resume ownership, identify any existing codec or registry outside the inspected source set, and define the later versioned checkpoint contract.
2. Track 01: define which core/state and scheduler fields a complete resumable checkpoint requires, distinct from the current entity-only `WorldSnapshot`.
3. Track 03: identify behavior/context registration state that must be included and its deterministic identity/lifecycle requirements.
4. Track 04: identify the RNG and seed state required for deterministic continuation.
5. Track 25: review the compatibility classification and API/release gates before any future public codec surface is added.

Until these owners agree on the portable checkpoint boundary, keep the Flow continuation context typed, owned, and in memory, and leave serialization and registration API choices to the later coordinated contract.

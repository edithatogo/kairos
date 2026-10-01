# ADR 0006: HPC and game runtime preview boundaries

Status: accepted for prototype integration; release acceptance remains pending.

## Context

Tracks 46–61 extend existing feature-gated Rust simulation modules and add ontology/code-generation and graph/game prototypes. Checked-in local evidence cannot establish live MPI, GPU, parallel filesystem, FMU execution or production scaling acceptance.

## Decision

Preserve the sequential engine, integer logical time, deterministic seeds, C ABI and telemetry schemas. Integrate the additional modules as explicitly bounded prototypes under their track contracts. Existing live hardware, release and registry evidence gates remain mandatory; integrating this PR does not make the unfinished tracks Done or declare their APIs stable.

FMU extraction creates a fresh private directory and rejects existing destinations; callers control its parent during extraction. Conservative logical processes begin with zero safe-time bounds for every declared peer and advance only after every peer promise permits it. Optimistic rollback preserves the committed state checkpoint below GVT when rebuilding retained history.

## Consequences

FMU callers must select a nonexistent output directory, rather than overlay an existing tree. Conservative transport users must exchange initial null-message bounds before expecting progress. Feature-gated contracts remain preview surfaces; consumer migration and external acceptance must be reassessed before beta/stable publication.

## Evidence and objections

Symlink destinations and preexisting resource symlinks are rejected by adversarial FMI tests. PDES fixtures cover missing/partial peer bounds and cancellation exactly at GVT after fossil collection. Their prior permissive behavior lost causality or committed state; preserving it was rejected. Test-only assumptions about unrestricted initial progress were replaced with explicit valid peer promises.

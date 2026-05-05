# Track 26: Interoperability Standards Review

## Purpose

Evaluate DEVS, FMI/FMU, SBML/CellML, OpenTelemetry, Arrow schema conventions, and mappings to existing simulation ecosystems. Maintain a clear inventory of supported, partial, deferred, and unsupported standards.

## Why this track exists

KairoECS lives in a broader simulation ecosystem. Without a deliberate standards review, the project risks making implied interoperability claims that it cannot honour, or missing alignment that would unlock adoption.

## Primary subagent

`interop-agent`

## Dependencies

- Track 00: Foundation — provides project identity and naming context.

## Owned paths

```text
docs/interoperability/, conductor/interoperability-standards.md
```

## Blocked paths

```text
crates/kairo-ecs-core/ — owned by Track 01 (core implementation)
crates/kairo-ecs-ffi/ — owned by Track 02 (FFI bridge)
```

## Inputs

- Core and FFI contracts from Tracks 01-02.
- Arrow schema contract from Track 04.

## Outputs

- Standards inventory mapping DEVS, FMI/FMU, SBML, CellML, OpenTelemetry, Arrow C Data Interface, Arrow IPC, and Parquet.
- Mapping table distinguishing supported, partial, deferred, and unsupported mappings.
- ADR recommendations for standards that would change public compatibility claims.
- Handoff notes for API governance and release planning subagents.

## Acceptance criteria

- Standards inventory names all 8 target standards explicitly.
- Mapping table labels each standard as supported / partial / deferred / unsupported.
- Release-impacting assertions (Arrow schema changes, semantic-convention drift) are named.
- Known gaps are documented with explicit missing behaviour.

## Non-goals

- Replacing the core scheduler or ECS design.
- Publishing packages before naming, legal, security, and compatibility gates pass.
- Adding domain-specific complexity to `kairo-ecs-core`.
- Implementing runtime support for unsupported standards.

## Release implications

- Arrow schema changes and semantic-convention drift require review before release language is used.
- Claims of external runtime interoperability must be backed by conformance fixtures.
- Unresolved mapping gaps are documented and accepted as deferred.

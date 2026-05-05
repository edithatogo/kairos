# Track 23: Domain Starter Kits & Model Zoo

## Purpose

Deliver practical example models and starter kits for health, logistics, manufacturing, transport, epidemics, and RL, showing users how to apply KairoECS to real domain problems.

## Why this track exists

KairoECS is a multi-language research and engineering ecosystem. Without concrete starter kits and a discoverable model zoo index, users cannot easily adapt the kernel to their domain. This track ensures example quality, maturity labelling, and discoverability so the project is approachable.

## Primary subagent

`model-zoo-agent`

## Dependencies

- Track 03: Flow/DES/ABM APIs — needed because starter kits consume domain model surfaces.
- Track 14: Documentation site — needed because model zoo lives on the docs surface.

## Owned paths

```text
examples/model-zoo/, docs/community/
```

## Blocked paths

```text
crates/kairo-ecs-core/ — owned by Track 01 (core implementation)
crates/kairo-ecs-des/ — owned by Track 03 (DES API)
crates/kairo-ecs-abm/ — owned by Track 03 (ABM API)
```

## Inputs

- Domain model examples from Tracks 03 (DES/ABM API shapes).
- Docs site layout from Track 14.

## Outputs

- Discoverable model-zoo index page on the docs site.
- Starter kit READMEs with maturity labels (alpha/beta/stable).
- Model-zoo YAML manifest with example paths, descriptions, and maturity status.
- Release gate row in `conductor/delivery-readiness-checklist.md` for example completeness.

## Acceptance criteria

- Model-zoo entry points are linked from the docs home page.
- Every starter kit README includes a maturity label and dependency list.
- At least one DES, one ABM, and one hybrid example run end-to-end before 1.0.
- Inventory checks in CI verify that referenced example paths exist.

## Non-goals

- Replacing the core scheduler or ECS design.
- Adding domain-specific complexity to `kairo-ecs-core`.
- Publishing domain models as standalone packages.

## Release implications

- Example maturity labels must be current before release notes are finalised.
- Broken example paths or missing model-zoo entries are a docs-blocker.

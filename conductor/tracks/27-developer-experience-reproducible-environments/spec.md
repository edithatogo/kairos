# Track 27: Developer Experience & Reproducible Environments

## Purpose

Provide devcontainers/Nix/devbox/bootstrap scripts so contributors can build the polyglot workspace and docs site consistently.

## Why this track exists

KairoECS is not only a Rust kernel. It is a multi-language research and engineering ecosystem. This track protects the project from the most common failure mode for ambitious open-source infrastructure: impressive internals with insufficient trust, examples, packaging, governance, and contributor experience.

## Primary subagent

`dx-agent`

## Parallelization model

This track is designed to run in parallel with core implementation. The subagent owns docs, policies, examples, checklists, manifests, fixtures, and automation controls. It must not block kernel development unless it identifies a release-blocking risk.

## Inputs

- `conductor/contracts/core-contract.md`
- `conductor/contracts/ffi-contract.md`
- `conductor/contracts/arrow-schema-contract.md`
- `conductor/contracts/conformance-contract.md`
- `conductor/package-ecosystem-plan.md`
- `reviews/red-team-report.md`

## Outputs

- Track-specific docs, templates, fixtures, examples, or workflow gates.
- Concrete contributor commands in `justfile`.
- Updated risk register where applicable.
- Handoff notes for adjacent subagents.
- Release-readiness criteria that can be checked automatically where possible.

## Blocked paths

```text
crates/ — owned by Tracks 01-05 (core implementation)
bindings/ — owned by Tracks 06-11 (language bindings)
```

## Acceptance criteria

- New contributor can run `just dev-validate` and see all toolchain versions green.
- `just docs-build` produces a working website.
- `.devcontainer/` opens and builds without manual steps.
- Bootstrap script completes without errors on a clean machine.

## Non-goals

- Replacing the core scheduler or ECS design.
- Publishing packages before naming, legal, security, and compatibility gates pass.
- Adding domain-specific complexity to `kairo-ecs-core`.
- Turning the docs site into a full docs framework when the repository already has a working local build path.




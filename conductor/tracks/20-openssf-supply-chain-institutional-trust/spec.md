# Track 20: OpenSSF, Supply Chain Trust & Institutional Readiness

## Purpose

Own the release-trust evidence needed to decide whether KairoECS can move from internal work to a public release.

## Why this track exists

KairoECS is not only a Rust kernel. It is a multi-language research and engineering ecosystem. This track protects the project from the most common failure mode for ambitious open-source infrastructure: impressive internals with insufficient trust, examples, packaging, governance, and contributor experience.

## Primary subagent

`security-agent`

## Parallelization model

This track is designed to run in parallel with core implementation. The subagent owns governance docs, release-trust evidence, and policy controls only. It must not change runtime code, public APIs, package behavior, or another worker's owned docs.

## Inputs

- `conductor/contracts/core-contract.md`
- `conductor/contracts/ffi-contract.md`
- `conductor/contracts/arrow-schema-contract.md`
- `conductor/contracts/conformance-contract.md`
- `conductor/package-ecosystem-plan.md`
- `reviews/red-team-report.md`

## Outputs

- A release-trust checklist that names the exact alpha, beta, RC, and 1.0 gates.
- A supply-chain evidence matrix covering signoff, scanning, provenance, and dependency controls.
- A short exception process for allowed-failure or not-yet-supported tooling.
- Handoff notes for release, CI, and red-team subagents.

### Third-party security audit

Before v1.0, KairoECS should either complete an independent security audit or
record an explicit release-blocking exception:

- Scope: scheduler determinism guarantees, FFI memory safety, Arrow deserialisation safety, supply-chain integrity (all dependencies audited).
- Auditor: OSTIF-affiliated or equivalent Rust security auditor.
- Funding: apply for OSTIF audit sponsorship or budget from institutional partners.
- Cadence: initial audit before v1.0; follow-up audit for any major architecture change (PDES, GPU, distributed).
- Public report: audit findings should be published under `docs/security/` once
  the audit exists. Do not claim an audit report before the file is checked in.

## Owned paths

- `conductor/tracks/20-openssf-supply-chain-institutional-trust/`
- `conductor/delivery-readiness-checklist.md`
- `conductor/quality-gates.md`

## Blocked paths

- Track 01 core scheduler and ECS implementation
- Track 02 FFI code generation and ABI behavior
- Track 06-11 binding implementation code
- Any package source tree outside the governance docs above

## Acceptance criteria

- A release manager can tell from the docs what blocks alpha, beta, RC, and 1.0.
- The artifact set distinguishes policy, evidence, and implementation.
- The track can be reviewed without reading implementation tracks.
- The track does not claim trust properties that are not captured by a gate or checklist.

## Release implications

- Missing `SECURITY.md`, `CODEOWNERS`, scorecard workflow, or SBOM/provenance plan blocks beta and later.
- Missing dependency review or vulnerability response path blocks RC and later.
- Any unresolved critical supply-chain issue becomes a release blocker until explicitly waived.

## Non-goals

- Replacing the core scheduler or ECS design.
- Publishing packages before naming, legal, security, and compatibility gates pass.
- Adding domain-specific complexity to `kairo-ecs-core`.
- Rewriting CI for unrelated implementation tracks.




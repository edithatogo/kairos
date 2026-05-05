# Track 28: Red Team & Devil's Advocate Review

## Purpose

Continuously attack the roadmap, architecture, governance, release process, and adoption strategy before users do, and turn the findings into release-blocker decisions.

## Why this track exists

KairoECS is not only a Rust kernel. It is a multi-language research and engineering ecosystem. This track protects the project from the common failure mode where impressive internals ship without a clear threat model, blocker list, or honest release language.

## Primary subagent

`redteam-agent`

## Parallelization model

This track is designed to run in parallel with core implementation. The subagent owns adversarial review notes, release-risk analysis, and blocker summaries only. It must not change runtime code, compatibility policy, or another worker's owned docs.

## Inputs

- `conductor/contracts/core-contract.md`
- `conductor/contracts/ffi-contract.md`
- `conductor/contracts/arrow-schema-contract.md`
- `conductor/contracts/conformance-contract.md`
- `conductor/package-ecosystem-plan.md`
- `reviews/red-team-report.md`

## Outputs

- A red-team report with named release blockers and counterexamples.
- A claim-versus-capability ledger for public docs and release notes.
- A list of surfaced risks mapped to the owning worker or gate.
- Handoff notes for release, security, docs, and API governance subagents.

## Owned paths

- `conductor/tracks/28-red-team-devils-advocate-review/`
- `reviews/red-team-report.md`
- `conductor/delivery-readiness-checklist.md`

## Blocked paths

- Track 20 supply-chain policy text
- Track 25 compatibility policy text
- Track 01 implementation code
- `conductor/contracts/` — changes to contracts require Track 25 signoff.
- Any package source tree outside the review docs above

## Acceptance criteria

- A release manager can see whether the track found an unresolved blocker.
- Every serious concern has an owner, severity, and follow-up path.
- The track distinguishes a genuine release blocker from a general concern.
- The output is specific enough to affect release planning.

## Release implications

- Any unresolved critical finding blocks beta, RC, and 1.0.
- Any claim in release notes that is not backed by a track artifact is a red-team concern.
- If the report is stale, release planning treats it as missing input.

## Non-goals

- Replacing the core scheduler or ECS design.
- Publishing packages before naming, legal, security, and compatibility gates pass.
- Adding domain-specific complexity to `kairo-ecs-core`.
- Owning the remediation work for the findings.




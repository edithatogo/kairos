# KairoECS Implementation Readiness

Last verified: 2026-05-05

## Purpose

The Conductor setup is complete, but implementation should move through explicit readiness levels. This avoids confusing "a workflow skipped because the package does not exist yet" with "the track is healthy", and it keeps GitHub and registry work tied to track maturity instead of guesswork.

## Readiness levels

| Level | Meaning | CI behavior |
|---|---|---|
| R0 | Planned only | Missing package manifests may skip. |
| R1 | Skeleton created | Owned directories and minimal docs exist. |
| R2 | First real package manifest exists | CI must run real smoke tests for that package. |
| R3 | Public API exists | Conformance fixtures are required. |
| R4 | Release candidate | Full quality, docs, package, SBOM, provenance, and red-team gates are required. |

## Current state

| Area | Readiness | Evidence |
|---|---|---| 
| Rust workspace | R2 | Root `Cargo.toml` and initial crates exist. |
| Core scheduler/types/state/RNG | R2 | `crates/kairo-ecs-*` skeleton crates compile as the first Track 01 slice. |
| Conformance fixtures | R1 | Initial JSON fixtures exist under `conformance/fixtures`. |
| Binding directories | R1 | Directories exist with README guards; package manifests are intentionally absent. |
| Docs site | R2 | `website/package.json` builds a static placeholder. |
| GitHub automation surface | R2 | `.github/` workflows, CODEOWNERS, dependency review, and release scaffolding exist. |
| Packaging | R1 | `packaging/README.md` exists; ecosystem package dirs wait for manifests. |
| GPU Compute (Track 32) | R0 | Planned only. |
| WebGPU Compute (Track 33) | R0 | Planned only. |
| PDES & Parallel Execution (Track 34) | R0 | Planned only. |
| Distributed Simulation (Track 35) | R0 | Planned only. |
| Streaming & Real-Time (Track 36) | R0 | Planned only. |
| ML/AI Integration (Track 37) | R0 | Planned only. |
| FMI/FMU Digital Twin (Track 38) | R0 | Planned only. |
| Cloud/HPC Batch Runners (Track 39) | R0 | Planned only. |
| Time-Travel Debugging (Track 40) | R0 | Planned only. |

## Enforcement rule

Once a track moves to `In Progress`, the files listed in `conductor/tracks.yaml` for that track must exist or be explicitly waived in the track handoff. Once an ecosystem package manifest is added, matching CI must fail on errors rather than skip.

Tracks cannot move to `In Review` or `Done` from planning text alone. A closeout must identify the owned files that exist in the worktree, the commands that exercised each required gate, and any waived gate with an owner and follow-up. For Tracks 32-40, `R0` means planning-only even if a handoff drafts future implementation language; implementation claims require matching owned paths and command evidence before status can advance.

## Immediate critical path

1. Track 00: finish naming/legal metadata and mark foundation done.
2. Track 01: complete `lanes.md` milestones for types, scheduler, state, RNG, and facade readiness.
3. Track 12: turn JSON fixtures into a shared runner.
4. Track 13: add a metadata validator for `conductor/tracks.yaml` and keep the existing GitHub workflows aligned with track metadata.
5. Track 14: replace the placeholder static docs build with the final docs stack.
6. Track 15: define the first registry/package dry-run sequence before any publish manifests are added.

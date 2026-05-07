# KairoECS Wave Policy

This policy is generated from the current dependency graph in `conductor/tracks.yaml`.
It prevents track work from being skipped or advanced without its direct and
transitive dependency evidence.

Validation command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\29-wave-manager-execution-gatekeeper\validate-wave-gates.ps1
```

Use `-ReportOnly` to print the derived wave membership, dependency closure, and
critical-path heatmap without returning a blocking exit code.

## Derived Wave Membership

Wave numbers are topological depths:

- A track with no dependencies is Wave 0.
- A track with dependencies is assigned to one plus the highest wave of its
  direct dependencies.
- If `conductor/tracks.yaml` changes, rerun the validator and update this
  section from the report.

Current snapshot generated on 2026-05-07 from `conductor/tracks.yaml`:

| Wave | Tracks |
|---:|---|
| 0 | 00 |
| 1 | 01, 12, 13, 14, 19, 26, 27, 28, 29 |
| 2 | 02, 03, 04, 05, 17, 18, 20, 30, 32, 34, 40 |
| 3 | 21, 22, 23, 25, 31, 35 |
| 4 | 06, 07, 08, 09, 10, 11, 16, 36, 37 |
| 5 | 15, 24, 33, 38 |
| 6 | 39 |

## Release Gatekeeper Tracks

- 29 Wave Manager & Execution Gatekeeper: owns `wave-progression-check` and `dependency-closure-check`.
- 30 Toolchain & Version Support Matrix: consumes the wave policy when toolchain gates are promoted.
- 31 Performance Regression Guard: consumes the wave policy when benchmark regression gates are promoted.

The current graph derives Wave 6 because Track 39 depends on Track 15, and Track
15 is already Wave 5. This replaces the previous fixed 0-5 assumption with a
graph-derived rule so the policy remains correct when the canonical track
inventory changes.

## Gate Rules

### No-skip controls

The wave gates are no-skip controls: report-only mode can document current
dependency blockers, but blocking mode must fail when a track advances before
its direct or transitive dependencies are `Done` or explicitly waived by an ADR.

1. Every track must appear exactly once in `conductor/tracks.yaml`.
2. Every track must have a declared owner and at least one required gate.
3. Every track directory must contain `spec.md`, `plan.md`,
   `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.
4. Every `depends_on` target must exist in the canonical track inventory.
5. Dependency cycles are blocking errors.
6. A track in `In Progress`, `In Review`, or `Done` must have every direct
   dependency marked `Done`.
7. A track in `In Progress`, `In Review`, or `Done` must have every transitive
   dependency marked `Done`.
8. `In Review` and `Done` handoffs must name changed files, commands run, gate
   results, and explicit waivers.
9. Release tracks must treat missing dependency evidence as a blocker, not as a
   future enhancement.

## Gate Definitions

### wave-progression-check

`wave-progression-check` enforces local readiness for each direct dependency.
It fails with the advancing track ID, dependency ID, and dependency status when a
track is beyond `Planned` and any direct dependency is not `Done`.

Example current blocker:

```text
Track 01 is In Progress but direct dependency 00 is Spec Approved, not Done.
```

### dependency-closure-check

`dependency-closure-check` computes the full transitive closure for each track.
It fails when any dependency in that closure is not `Done`, and it also reports
unknown dependencies and dependency cycles.

Example current blocker:

```text
Track 39 is In Progress but transitive dependency 15 is In Progress, not Done.
```

## Critical-Path Heatmap

The heatmap ranks tracks by the number of downstream tracks they gate through
the current dependency graph.

| Rank | Track | Wave | Direct dependents | Transitive dependents | Current status |
|---:|---:|---:|---:|---:|---|
| 1 | 00 | 0 | 13 | 40 | Spec Approved |
| 2 | 01 | 1 | 10 | 28 | In Progress |
| 3 | 12 | 1 | 14 | 21 | In Progress |
| 4 | 26 | 1 | 4 | 20 | In Progress |
| 5 | 04 | 2 | 9 | 14 | In Progress |
| 6 | 02 | 2 | 8 | 13 | In Progress |
| 7 | 25 | 3 | 7 | 11 | In Progress |
| 8 | 14 | 1 | 4 | 6 | In Progress |
| 9 | 09 | 4 | 3 | 4 | In Progress |
| 10 | 13 | 1 | 3 | 4 | In Progress |

## Exception Path

A maintainer override requires an ADR before release or merge signoff. The ADR
must name:

- the failed gate,
- the blocked track and dependency chain,
- the reason the dependency can be bypassed,
- the compensating control,
- the approving maintainer,
- the expiry or follow-up issue.

Overrides do not change `conductor/tracks.yaml`; they only document a human
exception to a failed gate.

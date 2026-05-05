# Agent Contract: wave-manager-agent

## Track

Track 29: Wave Manager & Execution Gatekeeper

## Owned paths

- `conductor/tracks/29-wave-manager-execution-gatekeeper/`
- `conductor/wave-policy.md`
- `conductor/gates/wave-progression-check.yml`
- `conductor/gates/dependency-closure-check.yml`
- Track-specific artifacts named in `plan.md`

## Required handoff

- Summary of wave assignments and any tracks that could not be cleanly assigned.
- Gate definitions and their pass/fail semantics.
- Critical-path heatmap.
- List of tracks currently blocking downstream tracks.
- Follow-up items for CI/CD (Track 13) and release (Track 15).

## Prohibited changes without ADR

- Modifying track status fields in `conductor/tracks.yaml`.
- Changing the status vocabulary (Planned, In Progress, Done, etc.).
- Reassigning track ownership or dependencies.
- Modifying CI/CD workflow files in `.github/workflows/`.
- Overriding a gate result without documented maintainer exception.
- Changing the wave assignment of a track without updating `conductor/wave-policy.md`.

## Gate contract

### wave-progression-check
- **Input**: `conductor/tracks.yaml`.
- **Output**: Pass if all tracks satisfy the rule "no track's status is beyond Planned when any of its `depends_on` are not Done". Fail with a list of violating tracks and their missing dependencies.
- **Blocking**: Yes — prevents merge of any PR that changes a track to "In Progress" with unsatisfied dependencies.

### dependency-closure-check
- **Input**: `conductor/tracks.yaml`.
- **Output**: Pass if every track's transitive dependency closure contains only "Done" tracks. Fail with the first unsatisfied transitive dependency chain.
- **Blocking**: Yes — prevents release packaging if any transitive dependency is incomplete.

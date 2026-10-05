# 13 CI/CD, Code Quality & Supply Chain — spec.md

## Mission

Implement GitHub Actions, quality gates, dependency updates, security scans, SBOM/provenance, and release automation skeleton.

## Primary subagent

```text
ci-agent + security-agent
```

## Dependencies

```text
Track 00. Starts immediately.
```

## Owned paths

```text
github workflows, deny.toml, codeql, renovate
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Accepted project identity and naming status where relevant.
- Relevant files under `conductor/contracts/`.
- Prior track handoff notes.

## Outputs

- Implementation in owned paths exists and is wired to the workspace.
- Tests or test-plan.
- Docs updates.
- Release notes or compatibility notes when public surfaces change.


## CI/CD scope

CI/CD must cover:

```text
Rust core quality
all language binding smoke tests
Python 3.10-3.14
C# .NET 10-11
Arrow roundtrip tests
docs site build
package dry-runs
security/supply-chain scans
scheduled Rust 1.99.0 mutation and benchmark checks; Miri and fuzzing remain UNVERIFIED because nightly execution is disallowed
release artifact creation
```

### Rust core routing contract

Core verification, workspace checks, and WebAssembly checks use the pinned
Rust 1.99.0 toolchain. Pull requests may skip these lanes only when a non-empty
diff consists entirely of known non-Rust paths. Empty diffs, or diffs containing
any Rust-relevant or unclassified path, run all three lanes. Every main push
runs them regardless of changed paths. The required `Rust core quality`
aggregate succeeds only when all three lanes pass for a Rust-required change
or all three are explicitly skipped for an allowed pull-request selection.
Missing or invalid classification fails the aggregate. The routing behavior is unchanged. The Rust policy now runs exact 1.99.0
workspace and WebAssembly checks, with Arrow telemetry checked in its own lane.



## Acceptance criteria

- Owned paths are created and documented.
- Contract inputs and outputs are explicit.
- Track tests or validation checks exist.
- CI gate is defined.
- Documentation impact is recorded.
- Release implications are recorded.
- `handoff.md` is completed before merge.


## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.

## Blocked paths

No additional blocked paths are declared for this track beyond the ownership and dependency boundaries in conductor/tracks.yaml. Public release, packaging, or production-readiness claims remain blocked until the relevant downstream release gates pass or are explicitly waived.


## Release implications

This track contributes to release readiness only through the acceptance criteria and quality gates listed here and in conductor/quality-gates.md. It does not independently authorize public release, registry publication, or production-readiness claims without the dependent packaging, supply-chain, compatibility, red-team, and wave-management gates.

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
github workflows, deny.toml, codeql, dependabot/renovate
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
nightly heavy checks
release artifact creation
```



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




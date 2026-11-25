# KairoECS Implementation Readiness

Use the [track registry](tracks.yaml) for workflow status, the
[capability register](capabilities.md) for supported claims, and the
[release gates](release-gates.md) for candidate acceptance. Find current
implementation and evidence in the track's plan, test matrix and handoff.
A workflow skipped because the package does not exist is not a pass.

## Readiness Levels

| Level | Meaning | CI behavior |
| --- | --- | --- |
| R0 | Planned only | Missing package manifests may skip. |
| R1 | Skeleton created | Owned directories and minimal docs exist. |
| R2 | First real package manifest exists | CI must run real smoke tests for that package. |
| R3 | Public API exists | Conformance fixtures are required. |
| R4 | Release candidate | Full quality, docs, package, SBOM, provenance, and red-team gates are required. |

## Enforcement Rule

Once a track moves to `In Progress`, its owned files must exist or have an
explicit waiver in the handoff. Once an ecosystem package manifest is added,
matching CI must fail on errors rather than skip.

Planning text cannot advance a track to `In Review` or `Done`. Closeout records
the owned files, commands that exercised the required gates, and any named
waiver with an owner and follow-up. R2 does not establish release readiness.

[Historical readiness snapshot at 9d76ca7](https://github.com/edithatogo/kairos/blob/9d76ca7f4b7f4b5898f2ec77515a7a7639f1478b/conductor/implementation-readiness.md).

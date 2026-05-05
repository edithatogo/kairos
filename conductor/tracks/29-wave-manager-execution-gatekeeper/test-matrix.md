# Test Matrix: Track 29 Wave Manager & Execution Gatekeeper

| Check | Alpha | Beta | RC | 1.0 |
|---:|---:|---:|---:|---:|
| Track docs exist and render cleanly | yes | yes | yes | yes |
| `conductor/wave-policy.md` exists and is consistent with `conductor/tracks.yaml` | yes | yes | yes | yes |
| Every track is assigned to exactly one wave (0-5) | yes | yes | yes | yes |
| Wave assignment is derivable from the dependency graph | yes | yes | yes | yes |
| `wave-progression-check` gate exists and is documented in `conductor/quality-gates.md` | yes | yes | yes | yes |
| `dependency-closure-check` gate exists and is documented in `conductor/quality-gates.md` | yes | yes | yes | yes |
| Gate fails when a track depends on a non-Done track | yes | yes | yes | yes |
| Gate passes when all dependencies are Done | yes | yes | yes | yes |
| Gate reports the specific track ID and missing dependency on failure | no | yes | yes | yes |
| Transitive dependency closure is validated | no | yes | yes | yes |
| Exception override path is documented with ADR requirement | no | yes | yes | yes |
| Critical-path heatmap is accurate and regenerated on status change | no | yes | yes | yes |
| No false positives: track with all deps Done is not blocked | no | yes | yes | yes |
| No false negatives: track with unsatisfied dep is blocked | no | yes | yes | yes |
| Dependency cycles are detected and reported as errors | no | yes | yes | yes |
| Gate blocks PR merge when violated | no | no | yes | yes |
| Wave policy is referenced in release governance (Track 16) | no | no | yes | yes |
| Override count is tracked and auditable | no | no | yes | yes |

# Risk Register — 03 The Flow: DES Trajectory API & ABM Behavior API

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| DES/ABM ownership split unresolved | 3 | 4 | 12 | Explicit ownership documented in ADR; shared kernel types from Track 01 | flow-agent | Duplicated APIs appear across DES and ABM without coordination |
| API design without conformance fixture coverage | 3 | 4 | 12 | Conformance fixtures in CI compare DES and ABM outputs against shared contracts | flow-agent | Conformance suite not passing before API freeze |
| Callbacks into host languages not performance-profiled | 4 | 3 | 12 | Benchmark callback dispatch latency; define acceptable overhead threshold | flow-agent | Callback overhead exceeds 2× native call |
| Cross-domain API inconsistency | 3 | 4 | 12 | Shared data model (Track 01 contracts); cross-track integration tests in CI | flow-agent | DES-only or ABM-only feature ships without partner API |

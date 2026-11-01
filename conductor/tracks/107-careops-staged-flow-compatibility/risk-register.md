# Track 107 risk register

| Risk | Mitigation | State |
|---|---|---|
| Narrow API is mistaken for the removed full FlowRuntime | Purpose-specific name, explicit exclusions, migration note | Open |
| Scheduler-driven release/promotion diverges from CareOps contract | Shared deterministic fixture and consumer-shaped conformance test | Open |
| Candidate changes seeded outputs | Base on Track106/SplitMix lineage; prohibit RNG changes | Controlled |
| Parent pin advances before acceptance | No parent pin paths owned; require separate R10.3 gates | Controlled |
| Prior zstd audit blocker | Dependency frontier PR #340 qualified the closure consumed by #334 | Resolved for source integration |

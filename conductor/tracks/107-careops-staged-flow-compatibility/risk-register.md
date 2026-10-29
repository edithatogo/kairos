# Track 107 risk register

| Risk | Mitigation | State |
|---|---|---|
| Narrow API is mistaken for the removed full FlowRuntime | Purpose-specific name, explicit exclusions, migration note | Open |
| Scheduler-driven release/promotion diverges from CareOps contract | Shared deterministic fixture and consumer-shaped conformance test | Open |
| Candidate changes seeded outputs | Base on Track106/SplitMix lineage; prohibit RNG changes | Controlled |
| Parent pin advances before acceptance | No parent pin paths owned; require separate R10.3 gates | Controlled |
| Existing workspace audit blocks hosted closeout | zstd 0.13.3 and zstd-safe 7.3.0 enter through Polars 0.45.1; lock is unchanged from base. Keep dependency remediation in a separate reviewed task and do not allowlist advisories | Open |

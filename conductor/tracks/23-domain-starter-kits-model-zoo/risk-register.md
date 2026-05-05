# Risk Register: Track 23 Domain Starter Kits & Model Zoo

| Risk | Likelihood | Impact | Mitigation | Owner |
|---|---|---|---|---:|---|
| Starter kit examples drift from actual crate APIs as kernel evolves | High | High | Pin example code versions in CI; run `cargo check` on every kit README before release | model-zoo-agent |
| Model zoo inventory becomes stale or unmaintained | Medium | High | Add CI inventory check that warns on missing example paths; require inventory update per release cycle | ci-agent |
| Scope creep: adding domain-specific logic to kits instead of using public APIs | Medium | Medium | Contract-first workflow — kits must only consume published `kairo-ecs-*` APIs | contracts-agent |
| Community-facing maturity labels claim stability that the API hasn't reached | Medium | High | Require conformance fixture parity before marking a kit as stable | docs-agent |
| Automation relies on unavailable tooling (e.g., CI runner lacks Julia/R toolchain) | Medium | Medium | Use dry-run/allowed-failure lanes until toolchain is stable | ci-agent |

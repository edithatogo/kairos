# Risk Register: Track 23 Domain Starter Kits & Model Zoo

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Starter kit examples drift from actual crate APIs as kernel evolves | 4 | 4 | 16 | Pin example code versions in CI; run `cargo check` on every kit README before release | model-zoo-agent | Any kit example fails `cargo check` |
| Model zoo inventory becomes stale or unmaintained | 3 | 4 | 12 | Add CI inventory check that warns on missing example paths; require inventory update per release cycle | ci-agent | Inventory check finds missing or broken paths |
| Scope creep: adding domain-specific logic to kits instead of using public APIs | 3 | 3 | 9 | Contract-first workflow — kits must only consume published `kairo-ecs-*` APIs | contracts-agent | Kit uses internal or unpublished API |
| Community-facing maturity labels claim stability that the API hasn't reached | 3 | 4 | 12 | Require conformance fixture parity before marking a kit as stable | docs-agent | Stability label applied without conformance fixture pass |
| Automation relies on unavailable tooling (e.g., CI runner lacks Julia/R toolchain) | 3 | 3 | 9 | Use dry-run/allowed-failure lanes until toolchain is stable | ci-agent | Required toolchain missing on CI after 2 release cycles |
| Starter-kit docs claim discoverability without a real example path | 2 | 4 | 8 | Keep starter-kit records in `examples/starter-kits/starter-kits.yaml` and validate every `example_paths` target with `examples/model-zoo/validate-inventory.ps1` | model-zoo-agent | Validator fails or a starter-kit row lacks a concrete example path |

## 2026-05-06 mitigation evidence

- Added `examples/model-zoo/validate-inventory.ps1` to check model-zoo paths, starter-kit paths, starter-kit README maturity/dependency sections, and starter-kit references to model-zoo ids.
- Added the first concrete starter-kit record: `manufacturing-bottleneck` -> `factory_bottleneck` -> `examples/des/factory_bottleneck`.

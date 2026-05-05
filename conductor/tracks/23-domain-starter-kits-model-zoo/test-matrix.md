# Test Matrix: Track 23 Domain Starter Kits & Model Zoo

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---|---:|---:|---:|
| Starter-kit inventory exists | yes | yes | yes |
| Model-zoo entry points are linked | yes | yes | yes |
| Starter-kit README maturity and dependency sections are present | yes | yes | yes |
| Markdown lint/link check | yes | yes | yes |
| Artifact existence check | yes | yes | yes |
| Docs build smoke test passes | yes | yes | yes |
| Release gate integration | no | yes | yes |
| Example or kit path is concrete | yes | yes | yes |
| Red-team objections about kit usefulness are answered | yes | yes | yes |
| Model-zoo docs are synchronised with YAML inventory | yes | yes | yes |

## 2026-05-06 local evidence

| Validation | Command | Result | Evidence |
|---|---|---|---|
| Model-zoo and starter-kit inventory/link check | `pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1` | Pass | Validated 4 model-zoo entries and 1 starter-kit entries. |
| Cross-track evidence-boundary guard | `node scripts/validation/validate-track21-27-evidence-boundaries.mjs` | Pass | Validates model-zoo docs/YAML synchronisation with the adjacent Track 21-27 evidence boundaries. |
| Concrete starter-kit path | `examples/starter-kits/starter-kits.yaml` | Pass | `manufacturing-bottleneck` maps to `examples/des/factory_bottleneck` and model-zoo id `factory_bottleneck`. |
| Tracks 21-27 aggregate smoke | `node scripts/validation/validate-tracks21-27.mjs` | Pass | Ran this inventory validator with the adjacent Track 21-27 local validators; all seven track checks passed. |
| Tutorial-ready model-zoo docs | `pwsh -NoProfile -File examples/model-zoo/validate-inventory.ps1` | Pass | Validator now requires model READMEs to include maturity, tutorial path, expected outputs, validation commands, and shared notebook/figure paths. |

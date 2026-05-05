# Test Matrix: Track 25 API Design Review & Compatibility Governance

| Check | Alpha | Beta | RC | 1.0 |
|---|---:|---:|---:|---:|
| Track docs exist and render cleanly | yes | yes | yes | yes |
| Surface inventory exists for `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng`, `bindings/python`, `bindings/r`, `bindings/julia`, `bindings/typescript`, `bindings/csharp`, and `bindings/go` | yes | yes | yes | yes |
| Compatibility policy names the live crate and package roots | yes | yes | yes | yes |
| `conductor/delivery-readiness-checklist.md` includes the compatibility gate rows | yes | yes | yes | yes |
| `conductor/quality-gates.md` includes the compatibility gate section | yes | yes | yes | yes |
| Breaking-change definition is explicit | no | yes | yes | yes |
| ADR requirement is explicit | no | yes | yes | yes |
| Migration note requirement is explicit | no | yes | yes | yes |
| Release-stage decision rules are explicit | no | yes | yes | yes |
| Release hold path is documented | no | yes | yes | yes |
| Package catalog and package matrix are aligned to the live binding/package roots | no | yes | yes | yes |
| Compatibility notes name the exact affected crate or package root | no | yes | yes | yes |
| Rename, split, or removal of a published root is treated as breaking | no | yes | yes | yes |
| Any public API, ABI, or schema change without an ADR is rejected | no | yes | yes | yes |
| Any breaking change without a migration note is rejected at beta and beyond | no | yes | yes | yes |
| Any root mismatch between policy and release docs is a release hold | no | yes | yes | yes |

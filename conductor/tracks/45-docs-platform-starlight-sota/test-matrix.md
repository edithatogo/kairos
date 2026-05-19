# 45 Astro/Starlight Docs Platform and Polyglot Experience - test-matrix.md

| Gate | Command | Expected result |
|---|---|---|
| Docs platform SOTA | `node scripts/validation/validate-docs-platform-sota.mjs` | Astro/Starlight, versioning, polyglot, llms.txt, icons, generated search, and archive-route evidence pass. |
| Website SOTA script | `npm --prefix website run check:sota` | Same SOTA validator passes through package script. |
| Full docs gate | `npm --prefix website run check:all` | Link validation, Starlight build, and quality validation pass. |
| Docs workflow smoke | `node scripts/dx/validate-docs-workflow.mjs` | Docs workflow and preview smoke pass. |
| Phase gate | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Non-terminal track metadata and closeout requirements pass. |
| Artifact shape | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_artifacts.ps1` | Required Track 45 artifacts are present. |
| Strict git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Requires clean tree after commit and push. |

## CI mapping

`.github/workflows/docs-quality.yml` runs `node scripts/validation/validate-docs-platform-sota.mjs` after `node scripts/dx/validate-docs-workflow.mjs`.

# 45 Astro/Starlight Docs Platform and Polyglot Experience - test-matrix.md

| Gate | Command | Expected result |
|---|---|---|
| Docs platform SOTA | `node scripts/validation/validate-docs-platform-sota.mjs` | Astro/Starlight, versioning, polyglot, llms.txt, icons, generated search, and archive-route evidence pass. |
| Website SOTA script | `npm --prefix website run check:sota` | Same SOTA validator passes through package script. |
| Full docs gate | `npm --prefix website run check:all` | Link validation, Starlight build, and quality validation pass. |
| Docs workflow smoke | `node scripts/dx/validate-docs-workflow.mjs` | Docs workflow and preview smoke pass. |
| Search DOM XSS regression | `node website/scripts/test-search-security.mjs` | Indexed text remains inert and non-HTTP(S)/cross-origin result URLs are rejected. Docs Quality runs this before the site build. |
| Phase gate | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Non-terminal track metadata and closeout requirements pass. |
| Artifact shape | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate_conductor_artifacts.ps1` | Required Track 45 artifacts are present. |
| Strict git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Requires clean tree after commit and push. |

## CI mapping

`.github/workflows/docs-quality.yml` runs `node scripts/validation/validate-docs-platform-sota.mjs` after `node scripts/dx/validate-docs-workflow.mjs`.

## Website cache source control — 2026-10-05

| Gate | Command | Expected result |
|---|---|---|
| Website adapter identity, drift and atomicity | `python3 -m unittest discover -s tests -p test_website_http_cache_patch.py -v` | Exact4.3 offline fixture patches to pinned output; idempotence succeeds; unsupported/mutated/symlink/mixed trees reject. |
| Raw website dependency audit | `npm audit --prefix website --audit-level=moderate --json` | Exit0; raw JSON remains retained separately from source mitigation. |
| Installed-source mitigation and behavior | `python3 website/scripts/apply_http_cache_fix.py website/node_modules` then `node tests/http-cache-security-regression.mjs website/node_modules/http-cache-semantics/index.js` | Pinned4.3 source becomes the reviewed local mitigation; all248 named cases pass. |

Docs and Docs Quality retain website-cache-security artifacts. A version-only audit pass is not source-security proof or release approval. Bootstrap EXC199 remains restricted to its original tree and scope.

| Released-source negative control | `node tests/http-cache-security-regression.mjs website/node_modules/http-cache-semantics/index.js --expect-vulnerable` before patching | Unsafe reuse is reproduced; this is a negative control, not acceptable installed behavior. |
| Complete retained evidence | `python3 website/scripts/verify_cache_security_evidence.py artifacts/website-cache-security` | Required receipts, raw audit graph/counts, nine source identities, copied package/license and current checkout agree; stale, missing, tampered or contradictory evidence rejects. |
| Combined offline controls | `python3 -m unittest discover -s tests -p 'test_website*cache*.py' -v` | 27 cases pass; includes contradictory receipts, hidden audit findings, symlinks and stale markers. |

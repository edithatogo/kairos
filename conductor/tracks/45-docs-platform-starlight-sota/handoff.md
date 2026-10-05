# 45 Astro/Starlight Docs Platform and Polyglot Experience - handoff.md

Last updated: 2026-06-23

## 2026-09-29 search-rendering security follow-up

The legacy generated documentation search client now builds result nodes with
DOM APIs and assigns indexed titles and excerpts through `textContent`. Result
URLs must resolve to same-origin HTTP(S); script schemes and external origins
are ignored. The regression harness runs in Docs Quality before the site build
and covers HTML payloads, dangerous schemes, cross-origin links, and relative
paths. This is a focused hardening change; it does not advance Track 45 status.

Validation on PR worktree based on `76820e7a0fbfe6306640b63f2ce55730d0fe7cbc`:

- `node website/scripts/test-search-security.mjs` — pass.
- `npm --prefix website run build` — pass; 16 Astro pages and 101 compatibility pages generated.
- `actionlint .github/workflows/docs.yml` — pass.
- `git diff --check` — pass.

## Summary

2026-05-19: Track 45 formalizes the active Astro/Starlight docs platform and adds a dedicated SOTA validator for versioning, the local polyglot plugin, llms.txt, icons, generated search, and archive-route evidence.

## Files changed

- `.github/workflows/docs-quality.yml`
- `docs/developer-experience/docs-platform.md`
- `website/package.json`
- `scripts/validation/validate-docs-platform-sota.mjs`
- `conductor/tracks/45-docs-platform-starlight-sota/*`
- Conductor registry/status surfaces for Track 45 ownership.

## Contracts consumed

- Track 14 docs build contract.
- Track 27 developer workflow and bootstrap contract.
- Track 41 docs workflow, learning coverage, and platform parity contract.
- Track 44 `>= 9.5` docs-health gate.

## Contracts changed

- Docs platform claims now require `docs-platform-sota`.
- The docs-quality workflow now validates the SOTA plugin stack.
- Deferred docs plugins are documented with activation conditions.

## Tests added

- `node scripts/validation/validate-docs-platform-sota.mjs`
- `npm --prefix website run check:sota`

## Known risks

- TypeDoc, OpenAPI, and hosted DocSearch remain deferred until source artifacts and operational decisions exist.
- The validator checks generated docs output, so `npm --prefix website run build` must run before standalone SOTA validation if build artifacts are stale.

## Follow-up issues

- Consider `starlight-typedoc` after TypeScript API reference generation is authoritative.
- Consider `starlight-openapi` after an OpenAPI contract exists.
- Consider hosted DocSearch only if Pagefind is insufficient for the public docs scale.

## Integration notes

Run `$conductor-review` before advancing this track. Apply accepted fixes in owned paths, record rejected fixes here, then run the test matrix.

## Phase closeout evidence

- `$conductor-review`: focused local review on 2026-06-18 found no Track 45 plan/spec defects in the Astro/Starlight platform gate. Deferred TypeDoc, OpenAPI, and hosted DocSearch remain correctly recorded as activation-condition follow-ups, not current requirements.
- accepted fixes: none required for the Track 45 owned surface in this pass.
- validation: `node scripts/dx/validate-docs-workflow.mjs` passed with link validation, Astro build, generated compatibility routes, and docs dev smoke; `node scripts/validation/validate-docs-platform-sota.mjs` passed with Starlight versioning, link validator, llms.txt, icons, and local polyglot plugin evidence.
- commit SHA: `0749d4139fff6a86cdf623c336541cd461055a9b`.
- pushed ref: `origin/codex/kairos-conductor-closeout` after branch push.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: passed on 2026-06-18 after restoring `origin/conductor-close-reviewed-tracks-20260510` to historical tip `a7e6f4a68bad9aa9483997d3a0207031066929a1`.
- next-phase decision: keep Track 45 `In Review` until pull-request CI confirms the branch.

## Archive review - 2026-06-23

- `$conductor-review`: focused archive review found no remaining in-scope source defects in the Track 45 active Astro/Starlight docs-platform surface.
- accepted fixes: archive/status bookkeeping only; no code-path fixes were required.
- validation: `npm --prefix website run check:sota` passed after the sandboxed run hit Windows `spawn EPERM`; `npm --prefix website run check:all` passed with link validation, Astro build, generated compatibility routes, and docs quality validation; `node scripts/dx/validate-docs-workflow.mjs` passed with docs dev smoke; Conductor phase-gate, DAG, and artifact validators passed with 0 errors and 0 warnings.
- residual scope: TypeDoc, OpenAPI, hosted DocSearch, and live hosted-search operations remain deferred activation-condition work and are not claimed by this archive.
- archive decision: Track 45 is `Done` for the repo-side active docs-platform gate.

- archive commit SHA: `c1ae99b516db2c7375508ff0b02d5536f385ecff`.
- pushed ref: `origin/codex/kairos-hpc-parity-wave` pending final push confirmation.

## Website dependency mitigation integration — 2026-10-05

Bounded follow-up to public advisory GHSA-ch52-4w7c-c8xp / alert69: website lock resolves http-cache-semantics4.3.0, with a separate website adapter retaining the existing hash-pinned source mitigation. Unmodified official4.3 fails189/248 security/compatibility cases; a composed scratch mitigation passes248/248. Version4.3 is not claimed as an official security fix. Bootstrap4.2 generator and EXC199 approval/source binding stay unchanged. Docs and Docs Quality invoke the website adapter, fail on raw npm audit at moderate severity, run its offline fail-closed tests and the actual248-case installed-source regression, and retain versions/source hashes/raw audit/logs in a distinct artifact. Final integrated source, actual install/build, independent review and exact-head hosted results remain required; this entry is an implementation handoff, not an executed-gate or deployment claim. No new policy exception or broader Track45/13 phase completion is asserted.

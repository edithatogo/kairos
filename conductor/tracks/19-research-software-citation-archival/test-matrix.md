# Test Matrix: Track 19 Research Software, Citation & Archival

| Check | Validation command | Required by alpha | Required by beta | Required by 1.0 |
|---|---|---:|---:|---:|
| Citation/archive metadata is internally consistent | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1` | yes | yes | yes |
| Citation metadata file exists and validates | `Test-Path CITATION.cff; rg -n "^cff-version:|^message:|^title:|^version:|^date-released:|^type:|^authors:|^abstract:|^keywords:|^license:|^repository-code:" CITATION.cff` | yes | yes | yes |
| Archive metadata seed exists | `Test-Path .zenodo.json; rg -n '"title"|"upload_type"|"version"|"publication_date"|"access_right"|"description"|"creators"|"license"|"keywords"' .zenodo.json` | yes | yes | yes |
| CodeMeta file exists and validates | `Test-Path codemeta.json; rg -n '"@context"|"codemeta-3.0"|"@type"|"name"|"description"|"version"|"datePublished"|"programmingLanguage"|"license"|"codeRepository"|"developmentStatus"' codemeta.json` | yes | yes | yes |
| Paper metadata matches citation target | `rg -n "^date:|KairoECS contributors|0.4.0-alpha.1|edithatogo/kairos" paper/paper.md paper/paper.bib` | yes | yes | yes |
| Archive note or release metadata exists | `rg -n "archive|release|citation|doi|Zenodo|0.4.0-alpha.1" docs/research/citation.md conductor/release-engineering.md conductor/package-catalog.md conductor/tracks/19-research-software-citation-archival/plan.md` | yes | yes | yes |
| Markdown lint/link check | `just check-docs` | yes | yes | yes |
| Artifact existence check | `Test-Path codemeta.json; Test-Path conductor/package-catalog.md` | yes | yes | yes |
| Docs build smoke test passes | `just docs-build` | yes | yes | yes |
| Release gate integration | `rg -n "citation|archiv|release|Zenodo|DOI|0.4.0-alpha.1" conductor/release-engineering.md conductor/tracks/19-research-software-citation-archival/handoff.md` | no | yes | yes |
| Citation guidance is explicit enough for reuse | `rg -n "CITATION.cff|codemeta|Zenodo|release notes|DOI|version" docs/research/citation.md conductor/tracks/19-research-software-citation-archival/handoff.md` | yes | yes | yes |
| Red-team objections about archival durability are answered | `rg -n "durability|archive|metadata|DOI|release note|repository URL" conductor/tracks/19-research-software-citation-archival/handoff.md docs/research/citation.md` | yes | yes | yes |
| Aggregate Track 12-20 evidence gate keeps citation metadata synchronized | `node tests/conformance/track12_20_evidence_check.mjs` | yes | yes | yes |

## Latest focused validation

Last local evidence recorded on 2026-05-11:

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1` -> pass; reported `version=0.4.0-alpha.1`, `repository=https://github.com/edithatogo/kairos`, and `archive_status=pre-release metadata seed, not yet DOI-minted`. The validator now normalizes SPDX license URLs before checking `codemeta.json` license alignment with `CITATION.cff`.
- `just check-docs` -> pass.
- `just docs-build` -> pass.
- `node tests/conformance/track12_20_evidence_check.mjs` -> pass for Track 19 inside the aggregate Track 12-20 evidence gate.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` -> pass.
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_dag.ps1` -> pass.
- Field presence checks for `CITATION.cff`, `.zenodo.json`, `codemeta.json`, and `paper/` metadata passed.
- `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` -> fail; working tree has uncommitted tracked or untracked changes, so strict closeout remains blocked until the tree is clean.

Review-hardening expectation:

- Re-run the validator after any edit to `CITATION.cff`, `codemeta.json`,
  `.zenodo.json`, `paper/`, `docs/research/citation.md`, or release notes.
  The current status must remain explicit: pre-release metadata seed, not yet
  DOI-minted.
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.


## Registry roadmap validation

- Run `just check-docs` for Markdown links and fragments.
- Run `pwsh -NoProfile -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1` to preserve existing citation metadata consistency.
- Run `pwsh -NoProfile -File scripts/validate_conductor_setup.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` to preserve numbered track structure and the existing ledger.
- Manually review the destination matrix against #90–#94: each destination must specify prerequisites, authoritative evidence and pending external state. Provider eligibility and completion require later native readbacks; local docs checks cannot prove them.

## Issue #91 metadata correction (2026-10-01)

The earlier 2026-05-11 date-bearing metadata result above is historical. Current
metadata is explicitly unreleased; publication dates are required only when the
lifecycle record switches to `released` with exact-release evidence. Field-grep
rows are discovery aids and do not establish release publication.

- `python3 conductor/tracks/19-research-software-citation-archival/test-metadata-lifecycle.py`: six isolated validator fixtures cover unreleased pass, rejected invented date, released missing evidence, locally consistent released fixture, date disagreement and crate-license drift.
- `pwsh -NoProfile -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1`: checks lifecycle, metadata agreement and Rust workspace license inheritance.
- `cargo metadata --no-deps --format-version 1`: resolved package license evidence; all 26 workspace crates inherit Apache-2.0 OR MIT.
- `node tests/conformance/track12_20_evidence_check.mjs`: aggregate assertion follows the same lifecycle contract.

A locally passing released fixture is deliberately synthetic and proves rejection
logic, not a published release. Issue #91 stays open for named-release readback.

## Registry assessments — #92–#94

Documentation-only changes use `node website/scripts/check-links.js`,
`pwsh -NoProfile -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1`,
JSON parsing of `docs/research/swh-evidence.json`, and `git diff --check`.
Hosted docs/quality checks on the exact PR head supply the build evidence.
Source/runtime suites need not be repeated locally for unchanged Rust source.
Manual review verifies primary-provider URLs, observation/source provenance,
explicit unknown states, separation of preparation/request/loading/resolution,
and that #92–#94 external gates remain open.

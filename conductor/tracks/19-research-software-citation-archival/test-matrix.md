# Test Matrix: Track 19 Research Software, Citation & Archival

| Check | Validation command | Required by alpha | Required by beta | Required by 1.0 |
|---|---|---:|---:|---:|
| Citation/archive metadata is internally consistent | `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1` | yes | yes | yes |
| Citation metadata file exists and validates | `Test-Path CITATION.cff; rg -n "^cff-version:|^message:|^title:|^version:|^date-released:|^type:|^authors:|^abstract:|^keywords:|^license:|^repository-code:" CITATION.cff` | yes | yes | yes |
| Archive metadata seed exists | `Test-Path .zenodo.json; rg -n '"title"|"upload_type"|"version"|"publication_date"|"access_right"|"description"|"creators"|"license"|"keywords"' .zenodo.json` | yes | yes | yes |
| CodeMeta file exists and validates | `Test-Path codemeta.json; rg -n '"@context"|"@type"|"name"|"description"|"version"|"datePublished"|"programmingLanguage"|"license"|"codeRepository"|"developmentStatus"' codemeta.json` | yes | yes | yes |
| Paper metadata matches citation target | `rg -n "^date:|KairoECS contributors|0.4.0-alpha.1|edithatogo/kairos" paper/paper.md paper/paper.bib` | yes | yes | yes |
| Archive note or release metadata exists | `rg -n "archive|release|citation|doi|Zenodo|0.4.0-alpha.1" docs/research/citation.md conductor/release-engineering.md conductor/package-catalog.md conductor/tracks/19-research-software-citation-archival/plan.md` | yes | yes | yes |
| Markdown lint/link check | `just check-docs` | yes | yes | yes |
| Artifact existence check | `Test-Path codemeta.json; Test-Path conductor/package-catalog.md` | yes | yes | yes |
| Docs build smoke test passes | `just docs-build` | yes | yes | yes |
| Release gate integration | `rg -n "citation|archiv|release|Zenodo|DOI|0.4.0-alpha.1" conductor/release-engineering.md conductor/tracks/19-research-software-citation-archival/handoff.md` | no | yes | yes |
| Citation guidance is explicit enough for reuse | `rg -n "CITATION.cff|codemeta|Zenodo|release notes|DOI|version" docs/research/citation.md conductor/tracks/19-research-software-citation-archival/handoff.md` | yes | yes | yes |
| Red-team objections about archival durability are answered | `rg -n "durability|archive|metadata|DOI|release note|repository URL" conductor/tracks/19-research-software-citation-archival/handoff.md docs/research/citation.md` | yes | yes | yes |

## Latest focused validation

Last local evidence recorded by Worker 2:

- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1` -> pass; reported `version=0.4.0-alpha.1`, `repository=https://github.com/edithatogo/kairos`, and `archive_status=pre-release metadata seed, not yet DOI-minted`.

Review-hardening expectation:

- Re-run the validator after any edit to `CITATION.cff`, `codemeta.json`,
  `.zenodo.json`, `paper/`, `docs/research/citation.md`, or release notes.
  The current status must remain explicit: pre-release metadata seed, not yet
  DOI-minted.

# Risk Register: Track 19 Research Software, Citation & Archival

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| CITATION.cff metadata stale | 3 | 4 | 12 | Local validator checks required fields and cross-file version/date/repository/title/license consistency before release-note edits | research-software-agent | `validate-citation-archive.ps1` fails |
| Zenodo DOI not reserved | 3 | 5 | 15 | Keep `0.4.0-alpha.1` explicitly marked as not yet DOI-minted until a Zenodo draft or DOI exists; reserve DOI at first public release | release-agent | Publication approaches without reserved DOI or draft link |
| JOSS paper not updated for release | 3 | 3 | 9 | Version paper alongside release; link paper revision to changelog | research-software-agent | Release candidate exists without corresponding paper revision |
| codemeta.json out of sync | 2 | 3 | 6 | Local validator compares CodeMeta version, date, and repository URL against `CITATION.cff` and `.zenodo.json`; CI wiring remains a later step | ci-agent | codemeta.json validation fails locally or in CI |
| Archival provenance not verifiable | 3 | 5 | 15 | Archive build provenance alongside artifact; include commit hash and build log in Zenodo record; current metadata seed does not claim minted archival provenance | research-software-agent | Zenodo record missing commit hash or build provenance |

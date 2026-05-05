# Risk Register: Track 19 Research Software, Citation & Archival

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| CITATION.cff metadata stale | 4 | 4 | 16 | Validate CITATION.cff against Cargo.toml authors and version in CI | research-software-agent | CI validation of CITATION.cff fails |
| Zenodo DOI not reserved | 3 | 5 | 15 | Reserve DOI at first public release; automate DOI minting in release pipeline | release-agent | Publication approaches without reserved DOI |
| JOSS paper not updated for release | 3 | 3 | 9 | Version paper alongside release; link paper revision to changelog | research-software-agent | Release candidate exists without corresponding paper revision |
| codemeta.json out of sync | 3 | 3 | 9 | Generate codemeta.json from CITATION.cff and Cargo.toml in CI; fail on mismatch | ci-agent | codemeta.json validation fails in CI |
| Archival provenance not verifiable | 3 | 5 | 15 | Archive build provenance alongside artifact; include commit hash and build log in Zenodo record | research-software-agent | Zenodo record missing commit hash or build provenance |

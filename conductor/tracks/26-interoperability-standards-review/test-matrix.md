# Test Matrix: Track 26 Interoperability Standards Review

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---|---:|---:|---:|
| Standards inventory names DEVS, FMI/FMU, SBML, CellML, OpenTelemetry, Arrow C Data Interface, Arrow IPC, and Parquet | yes | yes | yes |
| Mapping table distinguishes supported, partial, deferred, and unsupported mappings | yes | yes | yes |
| Markdown lint/link check | yes | yes | yes |
| Artifact existence check | yes | yes | yes |
| Docs build smoke test passes | yes | yes | yes |
| Release-impacting assertions are named for Arrow schema and semantic-convention drift | no | yes | yes |
| Known gaps are documented with explicit missing behavior | yes | yes | yes |
| Red-team objections about false interoperability claims are answered | yes | yes | yes |

## Validation commands

- `just docs-build`
- `just check-docs`
- `rg -n "DEVS|FMI/FMU|SBML|CellML|OpenTelemetry|Arrow C Data Interface|Arrow IPC|Parquet|supported|partial|deferred|unsupported" docs/interoperability/standards-review.md conductor/tracks/26-interoperability-standards-review`

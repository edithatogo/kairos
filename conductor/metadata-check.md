# Metadata Check Gate

Validates that root project metadata files are present and well-formed before any track proceeds beyond the foundation phase.

## Files validated

| File | Purpose | Required |
|---|---|---|
| `README.md` | Project description, build/usage instructions | Yes |
| `LICENSE.md` | Licensing terms with SPDX identifier | Yes |
| `CITATION.cff` | Machine-readable citation metadata | Yes |
| `codemeta.json` | Linked-data metadata (CodeMeta schema) | Yes |
| `.zenodo.json` | Zenodo archival metadata | Yes |

## Gate rules

- Each file must exist at the repository root.
- `LICENSE.md` must contain a valid SPDX identifier (e.g., `MIT`, `Apache-2.0`).
- `CITATION.cff` must validate against CFF schema v1.2.0 or later.
- `codemeta.json` must conform to the CodeMeta 3.0 crosswalk.
- `.zenodo.json` must pass `zenodo-schema` validation.

## CI integration

This gate is part of the `conductor/quality-gates.md` foundation-level checks. Tracks 00–05 must confirm this gate passes before entering their implementation phase.

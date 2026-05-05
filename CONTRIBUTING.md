# Contributing to KairoECS

Thanks for considering a contribution. KairoECS is intentionally contract-first because it spans Rust plus Python, R, Julia, TypeScript, C#, and Go.

## Start here

1. Read `README.md`.
2. Read `conductor/workflow.md`.
3. Pick a track in `conductor/track-map.md`.
4. Check owned paths in `conductor/subagents.md`.
5. Use the API review template for any public API/ABI/schema change.

## Required checks before PR

```bash
just fmt
just lint
just test
```

If the full environment is not ready yet, run the relevant subset and explain what could not be run.

## Public API changes

Public Rust APIs, C ABI signatures, Arrow schemas, and host-language APIs require:

- ADR
- conformance fixture update
- docs update
- compatibility assessment
- red-team objection response

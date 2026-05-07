# 00 Project Foundation, Governance & Naming — spec.md

## Mission

Initialize the KairoECS repository, governance model, licensing, naming due diligence, community standards, issue templates, and project automation skeleton.

## Primary subagent

```text
foundation-agent
```

## Dependencies

```text
None. Starts immediately.
```

## Owned paths

```text
README.md, LICENSE.md, governance/, docs/adr/, conductor/status.md, naming checklist
```

## Blocked paths

```text
.github/ — owned by Track 13 (CI/CD)
crates/ — owned by Tracks 01-05 (core implementation)
bindings/ — owned by Tracks 06-11 (language bindings)
```

## Parallel-safe with

```text
Tracks 13, 14, 16, 19, 20, 25, 26, 27, 28 — all start immediately
```

## Inputs

- Accepted project identity and naming status.
- Registry name search results (crates.io, PyPI, npm, NuGet, Julia, Go).
- Trademark availability assessment.

## Outputs

- Valid dual-license files (Apache-2.0 + MIT) in repository root.
- Naming due diligence report with confirmed available registry names.
- Governance directory template (CODE_OF_CONDUCT, CONTRIBUTING, etc.).
- ADR directory initialized with template.
- `conductor/status.md` reflecting current setup state.

## Acceptance criteria

- `LICENSE-APACHE` and `LICENSE-MIT` files exist with standard full text.
- `naming-due-diligence.md` registry checklist is complete with actual search results for all target registries.
- `governance/` directory exists with CODE_OF_CONDUCT, CONTRIBUTING, and maintainer docs.
- `docs/adr/` directory exists with at least one ADR template.
- `conductor/status.md` exists and accurately reflects repo state.
- `conductor/tracks.yaml` validation passes (schema check).
- Track CI gate is defined in `test-matrix.md`.


## Release implications

Track 00 establishes the repository foundation and governance surface required before any public release. It does not authorize registry publication, package signing, or production release claims; those remain gated by the later packaging, supply-chain, release-governance, and red-team tracks.


## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.




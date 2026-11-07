# Packaging

Track 15 owns release packaging dry-runs. Track 42 owns production publication
gates and registry submission workflows.

Create ecosystem-specific subdirectories only when the corresponding binding or native artifact has a real package manifest.

## Rust-only runtime gate

No package may advertise or activate a non-Rust simulation core. The old host
schedulers are retired, including in alpha artifacts. Python/TypeScript/C#
execution requires a built Rust native/Wasm artifact and actual execution
coverage; Go/R/Julia currently provide metadata/data surfaces without a
qualified runtime. See [binding support](../bindings/README.md) and the
[machine-readable runtime inventory](../bindings/runtime-policy.json).

Building an archive, importing a package, parsing a conformance fixture or
finding a library file does not satisfy native runtime acceptance. Required
native jobs must fail when their artifact is missing. Package publication
requires artifact provenance, ABI checks, an explicitly supported platform
matrix and consumer execution evidence. Dry-run manifests below are inventory
and packaging evidence only; they do not override these capability limits.

## R2 dry-run package inventory

`packaging/release-package-manifest.json` is the local source of truth for the
R2 packaging dry-run slice. It lists the checked-in Rust workspace package
surface and the six binding package surfaces, their first registry target,
fallback delivery path, and dry-run commands. The manifest is pinned to
`release_stage: r2-dry-run` and writes only these local outputs:

- `dist/release-artifact-manifest.json`
- `dist/SHA256SUMS`

Build the local release evidence without publishing:

```bash
python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run
```

This writes:

- `dist/release-artifact-manifest.json`
- `dist/SHA256SUMS`

Validate the inventory without writing outputs:

```bash
python packaging/scripts/build_release_manifest.py --check
```

Verify generated local evidence after the manifest builder writes `dist/`:

```bash
python packaging/scripts/build_release_manifest.py --verify-existing
```

## First local registry/package dry-run sequence

The first Track 15 sequence is intentionally offline, starts with the workspace
and binding package inventory check, and does not add publish manifests:

1. `python packaging/scripts/build_release_manifest.py --check`
2. `python packaging/scripts/build_release_manifest.py --version 0.0.0-r2-dry-run`
3. `python packaging/scripts/build_release_manifest.py --verify-existing`
4. `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1`

The validator rejects sequence steps that require network access, credentials,
uploads, or publish commands. Publish manifests remain out of scope until the
release gates explicitly allow them.

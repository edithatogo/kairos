# Packaging

Track 15 owns release packaging plans and dry-runs.

Create ecosystem-specific subdirectories only when the corresponding binding or native artifact has a real package manifest.

## R2 dry-run package inventory

`packaging/release-package-manifest.json` is the local source of truth for the
R2 packaging dry-run slice. It lists the checked-in Rust crate manifests and the
six binding manifests, their first registry target, fallback delivery path, and
dry-run commands.

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

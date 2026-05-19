# Release Helpers

This directory contains the reusable publication entrypoints used by the
protected registry workflows.

## Registry publication

```bash
node scripts/release/publish-registry.mjs --mode dry-run --ecosystem all --version 0.0.0-test
node scripts/release/publish-registry.mjs --mode publish --ecosystem go --version 0.1.0
```

Supported ecosystems:

- `rust`
- `python`
- `r`
- `julia`
- `typescript`
- `csharp`
- `go`

## HPC publication

```bash
node scripts/release/publish-hpc.mjs --mode dry-run --version 0.0.0-test
node scripts/release/publish-hpc.mjs --mode dry-run --version 0.0.0-test --with-docker
node scripts/release/publish-hpc.mjs --mode publish --version 0.1.0
```

The default dry-run is dependency-light and avoids Docker, admin, or live
platform access. Add `--with-docker` on a Docker-capable runner when container
build proof is available. The HPC helper preserves the live-runtime evidence
boundary: it can validate and build publication lanes, but it does not claim
production readiness for Kubernetes, Slurm, AWS Batch, GCP Batch, or Azure
Batch without recorded runtime proof.

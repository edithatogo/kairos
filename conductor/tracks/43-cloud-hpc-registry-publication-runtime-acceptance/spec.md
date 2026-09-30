# 43 Cloud/HPC Registry Publication & Runtime Acceptance - spec.md

## Mission

Implement the cloud/HPC publication lane for container images, Kubernetes bundles, Slurm templates, and AWS/GCP/Azure Batch assets.

Track 43 converts Track 39's scaffold/offline validation into a guarded publication and runtime-acceptance process. It must not claim production readiness until live runtime evidence exists.

## Primary subagent

```text
hpc-registry-agent + cloud-agent + release-agent
```

## Dependencies

```text
Tracks 22, 35, 39, 42, and 44.
```

## Owned paths

```text
.github/workflows/hpc-registry-publish.yml
packaging/hpc-registry-manifest.json
scripts/validation/validate-hpc-registry-readiness.mjs
conductor/tracks/43-cloud-hpc-registry-publication-runtime-acceptance/*
```

## Acceptance criteria

- OCI/container publication lane exists and requires digest, SBOM, and signature/attestation evidence.
- Kubernetes, Slurm, AWS Batch, GCP Batch, and Azure Batch lanes list live acceptance evidence requirements.
- Workflow defaults to validation/dry-run and gates public publication behind `hpc-publication`.
- Runtime claims remain blocked until live Docker, Kubernetes, Slurm, and provider canary evidence exists.
- Track 44 code/repo health `>= 9.5` is required before production publication.

## Release implications

Track 43 is release-gating for cloud/HPC registry publication and any production-ready cloud/HPC claim. It does not make Track 39 complete until live Docker, Kubernetes, Slurm, and provider canary evidence exists.

## Blocked paths

Production cloud/HPC publication remains blocked until live runtime evidence, protected environment approval, Track 44 health evidence, and release-manager signoff are recorded.

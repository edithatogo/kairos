# 43 Cloud/HPC Registry Publication & Runtime Acceptance - handoff.md

Last updated: 2026-06-18

## Summary

Track 43 adds publication and runtime-acceptance scaffolding for cloud/HPC release assets. Azure CPU Batch substrate proof exists, but live runtime proof remains pending for Docker, Kubernetes, Slurm, AWS/GCP provider canaries, Azure KairoECS container/scenario execution, and production publication.

## Files changed

- `.github/workflows/hpc-registry-publish.yml`
- `packaging/hpc-registry-manifest.json`
- `scripts/release/publish-hpc.mjs`
- `scripts/validation/validate-hpc-registry-readiness.mjs`
- `conductor/tracks/43-cloud-hpc-registry-publication-runtime-acceptance/*`

## Contracts consumed

- Track 39 runtime evidence boundary
- Track 42 package publication gate
- Track 44 code health floor

## Contracts changed

Cloud/HPC registry publication now requires Track 43 and Track 44 gates.

## Tests added

- `node scripts/validation/validate-hpc-registry-readiness.mjs`

## Known risks

Live Docker, Kubernetes, Slurm, AWS, and GCP execution contexts are unavailable in this workspace. Azure for Students can run a CPU Batch substrate canary, but it has no GPU/HPC-family quota and still lacks a readable `kairo-ecs-cli` image canary.

## Follow-up issues

- Run Docker image build/run canary.
- Run Kubernetes CRD/operator canary.
- Run Slurm single-job and array canaries.
- Run AWS/GCP Batch sandbox canaries.
- Run Azure `kairo-ecs-cli` container/scenario canary with output/checksum evidence.

## Partial runtime evidence

- 2026-05-20: Azure CPU Batch substrate canary passed. See `docs/cloud-hpc/azure-batch-canary-2026-05-20.md`.
- This partial evidence does not satisfy Track 43 runtime acceptance because no KairoECS image/scenario output, checksum, GPU/HPC proof, protected publication, or release-manager approval was recorded.

## Local environment probe

- 2026-06-18: read-only checks found no local `docker`, `kubectl`, `sbatch`, or `aws` command on `PATH`.
- 2026-06-18: `gcloud` has a configured project, but no Batch canary was run in this pass.
- 2026-06-18: `az account show` reports the Azure for Students subscription as enabled; no Azure mutation was run in this pass.

## Integration notes

Use the protected `hpc-publication` environment for public writes.

## Phase closeout evidence

`$conductor-review` must be run before promotion. Record accepted fixes, commit SHA, pushed ref, `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and next-phase decision here during closeout.

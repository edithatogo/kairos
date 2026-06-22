# 43 Cloud/HPC Registry Publication & Runtime Acceptance - handoff.md

Last updated: 2026-06-22

## Summary

Track 43 is In Review for the guarded publication and runtime-acceptance evidence slice. The registry manifest now points at a machine-readable runtime acceptance evidence manifest, and the readiness validator rejects unsupported production-ready evidence fixtures. Azure CPU Batch substrate proof exists, but live runtime proof remains pending for Docker, Kubernetes, Slurm, AWS/GCP provider canaries, Azure KairoECS container/scenario execution, and production publication.

## Files changed

- `.github/workflows/hpc-registry-publish.yml`
- `packaging/hpc-registry-manifest.json`
- `packaging/hpc-runtime-acceptance-evidence.json`
- `packaging/negative/hpc-runtime-evidence-production-without-proof.json`
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
- `node scripts/validation/validate-hpc-registry-readiness.mjs --check-negative-fixtures`
- `node scripts/release/publish-hpc.mjs --mode publish --version 0.0.0-test` must fail until live evidence is complete

## Known risks

Live Docker, Kubernetes, Slurm, AWS, and GCP execution contexts are unavailable in this workspace. Azure for Students can run a CPU Batch substrate canary, but it has no GPU/HPC-family quota and still lacks a readable `kairo-ecs-cli` image canary.

## Follow-up issues

- Run Docker image build/run canary.
- Run Kubernetes CRD/operator canary.
- Run Slurm single-job and array canaries.
- Run AWS/GCP Batch sandbox canaries.
- Run Azure `kairo-ecs-cli` container/scenario canary with output/checksum evidence.

## Runtime acceptance manifest

- `packaging/hpc-runtime-acceptance-evidence.json` records the six required Track 43 live scopes: Docker image CLI smoke, Kubernetes operator smoke, Slurm single/array smoke, AWS Batch canary, GCP Batch canary, and Azure Batch KairoECS canary.
- The manifest deliberately keeps `production_claim_status` as `blocked` until every required scope is `passed`.
- `packaging/negative/hpc-runtime-evidence-production-without-proof.json` proves the validator rejects a production-ready claim while required scopes remain pending.

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

`$conductor-review` status: requested for this slice after the runtime acceptance evidence gate implementation.

accepted fixes: publication-gate review found that protected publish mode could still push an OCI image while runtime evidence was blocked. The fix makes `publish-hpc.mjs --mode publish` require `validate-hpc-registry-readiness.mjs --check-negative-fixtures --require-live-publication-evidence`, and the protected workflow now uses the same strict gate before publication.

Local implementation/review commands for this slice:

- `node scripts/validation/validate-hpc-registry-readiness.mjs --check-negative-fixtures`
- `node scripts/release/publish-hpc.mjs --mode dry-run --version 0.0.0-test`
- `node scripts/release/publish-hpc.mjs --mode publish --version 0.0.0-test` failed as expected before Docker/GHCR with live-publication-evidence blockers.

next-phase decision: Track 43 is In Review for guarded publication and runtime evidence gating. Keep it out of Done until live Docker, Kubernetes, Slurm, AWS/GCP Batch, Azure KairoECS container/scenario evidence, protected publication, and release-manager approval are attached.

Record accepted review fixes, commit SHA, pushed ref, `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and GitHub Actions review after commit/push. Track 43 is not Done; production publication remains blocked by the live runtime evidence gaps above.

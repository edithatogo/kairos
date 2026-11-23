# Track 54 Handoff

Last updated: 2026-11-23

## Single-node Slurm replay

The single-job wrapper forwards an optional seed manifest. The seeded array
wrapper pairs scenario and seed inputs by index. Both submit arguments
directly to Slurm and preserve the legacy request-capture path.

Slurm 24.11.5 on a containerized Linux x86-64 NAS worker ran the single job
and two array tasks. All replay artifacts matched the direct CLI runs byte
for byte. An invalid scenario/seed pair failed without creating output.
Both legacy paths completed and emitted request captures. This qualifies
seed-manifest validation and scheduler-order replay on a local CPU worker.
Multi-node scheduling, Apptainer runtime qualification, live model restart,
and production storage remain open.
[Commands and checksums](evidence/slurm-single-node-replay.json)

## Summary

Track 54 owns live container, Slurm, Kubernetes, and provider runtime
acceptance. The archive below records the June offline baseline.

## Files changed

- `conductor/tracks/54-slurm-container-cloud-hpc-runtime-acceptance/*`
- `scripts/validation/validate-hpc-runtime-evidence.mjs`

## Contracts consumed

- Track 39 cloud/HPC scaffold.
- Track 43 publication acceptance.
- Track 49 MPI/gRPC runtime contract.
- Track 52 GPU runtime contract.
- Track 46 evidence manifest.

## Contracts changed

The runtime-evidence manifest now defines scheduler/provider scope records,
scenario-output checksum requirements for passed scopes, and structured blocker
fields for unavailable live environments.

## Tests added

Local validator coverage added:

- `node --check scripts/validation/validate-hpc-runtime-evidence.mjs`
- `node scripts/validation/validate-hpc-runtime-evidence.mjs`
- Negative fixture:
  `conductor/tracks/54-slurm-container-cloud-hpc-runtime-acceptance/negative/missing-checksum.json`

## Known risks from the June archive

No Docker, Kubernetes, Slurm, AWS, GCP, or Azure live KairoECS scenario proof
exists yet. The manifest records these as blocked scopes with owners,
expiration dates, and evidence commands instead of treating them as passed.

## Follow-up issues

- Run live container/Kubernetes/Slurm canaries.
- Run provider batch canaries when credentials/quota are available.
- Attach real scenario logs, raw outputs, and sha256 checksums before any Done claim.

## Integration notes

Track 55 consumes runtime evidence and scheduler metadata from this track.

## Phase closeout evidence

Run `$conductor-review`, record accepted fixes, commit SHA, pushed ref,
`validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`, and the
next-phase decision before advancing this track.

## Implementation Note 2026-06-23

Added local runtime evidence validation with blocked Docker, Kubernetes, Slurm, AWS Batch, GCP Batch, and Azure Batch scopes. Passed scopes must carry scenario output and sha256 checksums; non-passed scopes must carry structured blockers. Live runtime canaries remain pending.

Commands: node --check scripts/validation/validate-hpc-runtime-evidence.mjs; node scripts/validation/validate-hpc-runtime-evidence.mjs.

## Archive review - 2026-06-23

- `$conductor-review`: focused archive review found no remaining in-scope source defects in the Track 54 local blocked-scope runtime-evidence gate.
- accepted fixes: archive/status bookkeeping and phase-closeout state repair; no validator logic fixes were required.
- validation: `python cloud/validate_cloud_hpc.py` passed with the documented static shell fallback, `node scripts/validation/validate-hpc-registry-readiness.mjs` passed with `production_claim_status: blocked`, `node --check scripts/validation/validate-hpc-runtime-evidence.mjs`, `node scripts/validation/validate-hpc-runtime-evidence.mjs`, Conductor phase-gate, DAG, artifact validators, and `git diff --check` passed locally.
- residual scope: Docker image execution, Kubernetes server-side validation, Slurm jobs, AWS/GCP/Azure Batch canaries, real Track 49 MPI runtime paths, and real Track 52 GPU runtime paths remain incomplete. This archive does not claim production cloud/HPC runtime acceptance.
- archive decision: Track 54 is `Done` for the repo-side blocked-scope runtime-evidence gate only.

- archive commit SHA: `e969bbef99836253d361bfb4c7546f583c74d002`.
- pushed ref: `origin/codex/kairos-hpc-parity-wave`.

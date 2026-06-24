# Track 55 Handoff

Last updated: 2026-06-23

## Summary

Track 55 owns final weak/strong scaling certification for HPC parity. It is
now In Progress with a first scenario and evidence-manifest contract slice.
This does not claim live weak scaling, strong scaling, or production HPC
parity certification. The new Track 55 HPC evidence manifests split local
smoke/contract validation from live weak and strong scaling proof templates.

## Files changed

- `conductor/tracks/55-end-to-end-weak-strong-scaling-certification/*`
- `conductor/hpc-evidence/manifests/track55-local-scaling-smoke-scaffold.json`
- `conductor/hpc-evidence/manifests/track55-live-weak-scaling-template.json`
- `conductor/hpc-evidence/manifests/track55-live-strong-scaling-template.json`
- `docs/benchmarks/hpc-scaling-certification.md`
- `scripts/validation/validate-hpc-scaling-certification.mjs`

## Contracts consumed

- Tracks 47-54 production runtime and evidence contracts.
- Track 46 evidence manifest.
- Tracks 18 and 31 benchmark policy.
- Tracks 42-44 publication and health gates.

## Contracts changed

Added the Track 55 certification-contract shape:

- `scenarios.json` defines representative DES, ABM, hybrid, distributed,
  MPI/gRPC, NUMA/I/O, FMI, and scheduler scenario coverage.
- `evidence.json` defines draft weak/strong profiles, accepted raw-result
  reference schemes, checksum requirements, active blockers, and the canonical
  Track 55 HPC evidence scaffold/template paths.
- `track55-local-scaling-smoke-scaffold.json` records the local-only contract
  and benchmark smoke gate without live-HPC proof.
- `track55-live-weak-scaling-template.json` and
  `track55-live-strong-scaling-template.json` define the fields that must be
  replaced with real pushed-commit, raw-artifact, checksum, hardware,
  scheduler, storage, toolchain, reviewer, and no-waiver evidence before a
  profile can be certified.
- `validate-hpc-scaling-certification.mjs` rejects malformed scenario
  coverage, missing weak/strong profiles, certified profiles without raw
  results, bad checksums, unsupported raw-result artifact schemes, incomplete
  certified-profile category coverage, and unsupported claim language.

## Tests added

- `node --check scripts/validation/validate-hpc-scaling-certification.mjs`
- `node scripts/validation/validate-hpc-scaling-certification.mjs --self-test`
- `node scripts/validation/validate-hpc-parity-evidence.mjs`

## Known risks

No integrated weak or strong scaling evidence exists yet. Tracks 47-54 still
need live runtime evidence before Track 55 can certify any production HPC
parity claim.

## Follow-up issues

- Replace draft blockers with live raw-result manifests after upstream tracks
  close.
- Run weak and strong scaling profiles on live HPC resources.
- Replace the weak and strong live templates with actual `live-hpc` manifests
  and `waiver.status: none`.
- Update release claims only after evidence review.

## Integration notes

Release and registry publication tracks must consume this certification before
making production HPC parity claims.

## Phase closeout evidence

- Red TDD command:
  `node scripts/validation/validate-hpc-scaling-certification.mjs` failed
  because the validator did not exist.
- Green contract commands:
  `node --check scripts/validation/validate-hpc-scaling-certification.mjs`
  passed.
  `node scripts/validation/validate-hpc-scaling-certification.mjs --self-test`
  passed.
- Benchmark smoke:
  `python benches/benchmark_smoke.py` passed.
- Regression threshold coverage:
  `python benches/regression/compare.py` passed.
- HPC evidence manifest shape:
  `node scripts/validation/validate-hpc-parity-evidence.mjs` passed for the Track 55 local scaffold and live weak/strong templates.
- `$conductor-review`: read-only review completed for this slice.
- accepted fixes: repaired the scaling validator syntax, enforced accepted
  raw-result artifact schemes, and required certified weak/strong profiles to
  cover every required scenario category.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: pending
  until after this task commit.
- implementation commit SHA: `dcefb0feea897688d13cd8905c157a4759246015`.
- evidence commit SHA: `c99b154748b68f7146fecd8ab7d5106a46a34ae4`.
- validator repair commit SHA: pending.
- pushed ref: `origin/codex/kairos-hpc-parity-wave`.
- next-phase decision: remain In Progress until live weak/strong scaling
  profiles and upstream Tracks 47-54 evidence close.

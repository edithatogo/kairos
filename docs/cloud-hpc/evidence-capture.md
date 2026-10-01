# HPC Evidence Capture

Use `scripts/evidence/capture-hpc-evidence.mjs` when a free runner, container,
Hugging Face Space, Slurm node, GPU host, or parallel filesystem target becomes
available. The tool runs one command, stores the raw log, computes a SHA-256
checksum, and emits a Conductor HPC evidence manifest.

Local and free-runner captures should stay `evidence_class: scaffold` unless the
run is the actual live target required by the track. A `live-hpc` manifest must
record a pushed commit, raw artifact checksum, reviewer, no active waiver, and
the real hardware, scheduler, filesystem, toolchain, and runtime command under
test.

Example scaffold capture on a free GitHub-hosted runner:

```bash
node scripts/evidence/capture-hpc-evidence.mjs \
  --track-id 54 \
  --task-id free-runner-docker-smoke \
  --capability docker-container-on-github-actions \
  --out conductor/hpc-evidence/captures \
  --feature-flags none \
  --expected "container CLI help exits successfully" \
  -- docker run --rm kairo-ecs-cli:cloud-smoke --help \
  -- docker run --rm kairo-ecs-cli:cloud-smoke --help
```

Example live Slurm capture once a scheduler is available:

```bash
node scripts/evidence/capture-hpc-evidence.mjs \
  --track-id 54 \
  --task-id live-slurm-single-job \
  --evidence-class live-hpc \
  --capability slurm-single-job-submission \
  --out conductor/hpc-evidence/captures \
  --scheduler "Slurm <version>" \
  --filesystem "Lustre <mount and stripe config>" \
  --reviewer "<reviewer>" \
  -- hpc/slurm/submit-experiment.sh --scenario scenarios/factory_bottleneck_v1.yaml --output /scratch/kairo-ecs-runs --partition gpu --nodes 1 \
  -- hpc/slurm/submit-experiment.sh --scenario scenarios/factory_bottleneck_v1.yaml --output /scratch/kairo-ecs-runs --partition gpu --nodes 1
```

After reviewing the generated JSON, move accepted live manifests into
`conductor/hpc-evidence/manifests/` and run:

```bash
node scripts/validation/validate-hpc-parity-evidence.mjs
node scripts/validation/validate-hpc-live-template-blockers.mjs --self-test
```

# Slurm Batch Execution

See the [runtime evidence boundary](runtime-evidence-boundary.md) for qualified scheduler configurations.

## Offline validation

Activate the repository's Python environment and run
`python cloud/validate_cloud_hpc.py` from the repository root. The validator
checks the wrappers and generated batch scripts with `bash -n`, plus the
signal, job-array, and checkpoint command wiring. Where Bash cannot start,
the validator checks quoting, heredoc closure, shebangs, line endings, and
common shell block balance.

## Live validation

The offline check does not prove a site scheduler accepts the generated scripts. Before marking a cluster ready, run `sbatch --test-only` if supported, otherwise submit a one-task canary to a short/debug partition and record the cluster, partition, job id, terminal status, and checkpoint/output paths.

Submit a single experiment with a TOML scenario and its seed manifest. Both
inputs and the output directory must be accessible to the batch task.

```bash
KAIRO_CHECKPOINT_DIR="/scratch/$USER/kairo/run-001/checkpoints" \
  bash hpc/slurm/submit-experiment.sh \
    --scenario examples/experiments/factory_bottleneck_v1.scenario.toml \
    --seed-manifest examples/experiments/factory_bottleneck_v1.seeds.toml \
    --output "/scratch/$USER/kairo/run-001" --partition cpu --nodes 1
```

With a seed manifest the CLI validates the pair and writes replay outputs,
including `summary.json`. A submission without a seed manifest writes the
legacy `run-request.json` capture. Use the site's partition and make
`kairo-ecs-cli` available on the worker. On sites with Apptainer, the single-job
wrapper uses the configured `--image`. Make the inputs and output path
accessible inside the container.

Use a seeded Slurm array for parameter sweeps. Each array task reads
`variant-<index>.scenario.toml` from `KAIRO_SCENARIO_PREFIX` and
`variant-<index>.seeds.toml` from `KAIRO_SEED_MANIFEST_PREFIX`. It writes to
`variant-<index>` inside the output directory.

```bash
KAIRO_SCENARIO_PREFIX=scenarios/sweep KAIRO_SEED_MANIFEST_PREFIX=scenarios/sweep \
  KAIRO_OUTPUT_URI="/scratch/$USER/kairo/sweep-001" KAIRO_SWEEP_SIZE=2 \
  bash hpc/slurm/submit-sweep.sh
```

Without `KAIRO_SEED_MANIFEST_PREFIX`, the array uses the legacy
`variant-<index>.yaml` request path.

The batch scripts request `--signal=B:SIGTERM@120` and call the CLI checkpoint
command when the batch shell handles the signal. The CLI records checkpoint
and resume requests. A model-owned checkpoint is required to restore live
simulation state.

```bash
KAIRO_OUTPUT_URI="/scratch/$USER/kairo/resumed" \
  bash hpc/slurm/resume.sh "/scratch/$USER/kairo/run-001/checkpoints/checkpoint-manifest.json"
```

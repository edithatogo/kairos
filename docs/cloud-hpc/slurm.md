# Slurm Batch Execution

See `runtime-evidence-boundary.md` for the live evidence gate and pending blocker status before any readiness claim.

## Offline validation

Run `python cloud\validate_cloud_hpc.py` from the repository root. The offline validator checks Slurm script syntax with `bash -n` when Bash starts successfully, confirms `--signal=B:SIGTERM@120`, validates job-array index wiring, and confirms checkpoint/resume command wiring. On Windows hosts where Bash is present but cannot start, the validator runs a limited static fallback over the shell scripts for quoting, heredoc closure, shebangs, line endings, and common block balance; this is useful as a guardrail but is not equivalent to `bash -n`.

## Live validation

The offline check does not prove a site scheduler accepts the generated scripts. Before marking a cluster ready, run `sbatch --test-only` if supported, otherwise submit a one-task canary to a short/debug partition and record the cluster, partition, job id, terminal status, and checkpoint/output paths.

Submit a single experiment by running the wrapper, which writes a temporary
batch script and invokes `sbatch` internally:

```bash
hpc/slurm/submit-experiment.sh --scenario scenarios/factory.yaml --output /scratch/$USER/kairo/run-001 --partition cpu --nodes 1
```

For GPU queues, pass the site-specific GPU partition:

```bash
hpc/slurm/submit-experiment.sh --scenario scenarios/factory.yaml --output /scratch/$USER/kairo/gpu-run --partition gpu --nodes 1
```

Parameter sweeps use Slurm job arrays. Set `KAIRO_SCENARIO_PREFIX`, `KAIRO_OUTPUT_URI`, and `KAIRO_SWEEP_SIZE`, then run `hpc/slurm/submit-sweep.sh`; the wrapper writes an array script and invokes `sbatch` internally. Each array task reads `variant-$SLURM_ARRAY_TASK_ID.yaml`.

The generated batch script requests `--signal=B:SIGTERM@120`. On preemption, the job traps `SIGTERM` and calls `kairo-ecs-cli checkpoint` before Slurm terminates the allocation. Resume with:

```bash
KAIRO_OUTPUT_URI=/scratch/$USER/kairo/resumed hpc/slurm/resume.sh /scratch/$USER/kairo/run-001/checkpoints/checkpoint-manifest.json
```

### Runtime evidence status

- This doc is paired with `runtime-evidence-boundary.md` for the explicit "not-yet-run" blocker records.

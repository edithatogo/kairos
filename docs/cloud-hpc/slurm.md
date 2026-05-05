# Slurm Batch Execution

Submit a single experiment with:

```bash
sbatch hpc/slurm/submit-experiment.sh --scenario scenarios/factory.yaml --output /scratch/$USER/kairo/run-001 --partition cpu --nodes 1
```

For GPU queues, pass the site-specific GPU partition:

```bash
sbatch hpc/slurm/submit-experiment.sh --scenario scenarios/factory.yaml --output /scratch/$USER/kairo/gpu-run --partition gpu --nodes 1
```

Parameter sweeps use Slurm job arrays. Set `KAIRO_SCENARIO_PREFIX`, `KAIRO_OUTPUT_URI`, and `KAIRO_SWEEP_SIZE`, then run `hpc/slurm/submit-sweep.sh`. Each array task reads `variant-$SLURM_ARRAY_TASK_ID.yaml`.

The generated batch script requests `--signal=B:SIGTERM@120`. On preemption, the job traps `SIGTERM` and calls `kairo-ecs-cli checkpoint` before Slurm terminates the allocation. Resume with:

```bash
KAIRO_OUTPUT_URI=/scratch/$USER/kairo/resumed hpc/slurm/resume.sh /scratch/$USER/kairo/run-001/checkpoints/checkpoint-manifest.json
```

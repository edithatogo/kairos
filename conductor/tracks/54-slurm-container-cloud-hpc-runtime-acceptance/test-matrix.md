# Track 54 Test Matrix

| Gate | Command | Required for |
|---|---|---|
| Offline cloud/HPC validation | `python cloud/validate_cloud_hpc.py` | Review |
| HPC registry readiness | `node scripts/validation/validate-hpc-registry-readiness.mjs` | Review |
| Runtime evidence manifest | `node scripts/validation/validate-hpc-runtime-evidence.mjs` | Review |
| Free compute route boundary | `node scripts/validation/validate-free-compute-routes.mjs` | Review |
| Docker runtime | `docker build -t kairo-ecs:hpc . && docker run --rm kairo-ecs:hpc kairo-ecs-cli --version` | Done |
| Kubernetes runtime | `kubectl apply --dry-run=server -f k8s/` | Done |
| Slurm runtime | `sbatch hpc/slurm/kairo-ecs-smoke.sbatch` | Done |
| Phase gates | `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Phase movement |
| Git closeout | `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Closeout |

Strict closeout requires `RequireCleanWorkingTree` after each task commit.

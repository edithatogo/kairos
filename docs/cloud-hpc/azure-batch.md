# Azure Batch

See `runtime-evidence-boundary.md` for required live evidence and pending blocker status before any production-readiness claim.

`cloud/azure/batch-job.json` and `cloud/azure/batch-array.json` provide account-neutral job templates. The submit helper expects an existing Batch pool and a container image readable from the pool.

The current student-account capability check is recorded in
`azure-student-capability.md`. As of 2026-05-20, the signed-in Azure for
Students subscription is enabled, and the required `Microsoft.Batch`,
`Microsoft.ContainerRegistry`, `Microsoft.ContainerInstance`, and
`Microsoft.Storage` providers are registered. A live CPU Batch substrate canary
was completed on 2026-05-20; see `azure-batch-canary-2026-05-20.md`.

## Offline validation

Run `python cloud\validate_cloud_hpc.py` from the repository root. The offline validator parses the Azure JSON templates and checks job termination policy, sweep metadata, submit-helper use of `az batch job create`, task creation, container image wiring, and `KAIRO_OUTPUT_URI` environment propagation.

## Live validation

The offline check is not an Azure Batch API validation. A CPU substrate canary
now proves that the student subscription can create a Batch account, allocate a
low-priority Ubuntu node, run a task, and return stdout/stderr. Before marking
Azure Batch ready for KairoECS, run `cloud/azure/submit-experiment.ps1` against
a readable `kairo-ecs-cli` container image and record the account, pool, job id,
task id, terminal status, output URI, and checksum evidence.

### Runtime evidence status

- CPU Batch substrate canary: pass, 2026-05-20.
- KairoECS container/scenario execution: pending.
- GPU/HPC hardware proof: blocked in the student subscription by zero GPU/HPC
  Batch quota.

Required permissions:

- Batch job create
- Batch task create
- Write access to the target Azure Blob container

Submit a run:

```powershell
$env:AZURE_BATCH_POOL_ID = "kairo"
$env:KAIRO_IMAGE = "registry.example/kairo-ecs-cli:latest"
$env:KAIRO_SCENARIO = "https://storage/scenario.yaml"
$env:KAIRO_OUTPUT_URI = "az://container/output/run-001"
cloud/azure/submit-experiment.ps1 -JobId kairo-run-001
```

# Azure Batch

`cloud/azure/batch-job.json` and `cloud/azure/batch-array.json` provide account-neutral job templates. The submit helper expects an existing Batch pool and a container image readable from the pool.

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

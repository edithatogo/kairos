# Azure Student Capability Check

Last checked: 2026-05-20.

This note records what can and cannot be completed with the locally signed-in
Azure for Students account.

## Account

- Active Azure CLI account: student Azure account verified locally.
- Subscription: `Azure for Students`.
- Subscription state: enabled.
- Visible region: `australiaeast`.
- Disposable canary resource group: `rg-kairos-batch-canary-20260520`.

## Provider readiness

The following resource providers were checked read-only:

| Provider | Status | Impact |
|---|---|---|
| `Microsoft.Batch` | `Registered` | Azure Batch canary setup can proceed. |
| `Microsoft.ContainerRegistry` | `Registered` | Azure Container Registry setup can proceed. |
| `Microsoft.ContainerInstance` | `Registered` | ACI smoke setup can proceed if needed. |
| `Microsoft.Storage` | `Registered` | Storage account/container setup can proceed. |

This means the account is useful for planning, template validation, and Azure
resource setup. A live CPU Azure Batch substrate canary was completed on
2026-05-20 and is recorded in `azure-batch-canary-2026-05-20.md`.

## Current runtime boundary

The completed canary proves that the subscription can create a disposable Batch
account, allocate a low-priority CPU node, run a task, and return output.

It does not yet prove KairoECS container/scenario execution, GPU parity, HPC
scaling, or production registry acceptance. The Batch account quota report
showed zero quota for GPU/HPC VM families, so GPU/HPC hardware testing remains
blocked in this subscription unless quota is granted or another runner is used.

The next Azure step is to make a readable `kairo-ecs-cli` image available to
the Batch pool, then run the smallest factory-bottleneck canary through
`cloud/azure/submit-experiment.ps1` and record output/checksum evidence.

The canary pool was resized back to zero nodes after the run. To remove all
disposable resources after evidence review:

```powershell
az group delete --name rg-kairos-batch-canary-20260520 --yes --no-wait
```

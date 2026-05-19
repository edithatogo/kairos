# Azure Student Capability Check

Last checked: 2026-05-20.

This note records what can and cannot be completed with the locally signed-in
Azure for Students account without making a live provider claim.

## Account

- Active Azure CLI account: student Azure account verified locally.
- Subscription: `Azure for Students`.
- Subscription state: enabled.
- Visible region: `australiaeast`.
- Resource group details: redacted from public documentation.

## Provider readiness

The following resource providers were checked read-only:

| Provider | Status | Impact |
|---|---|---|
| `Microsoft.Batch` | `Registered` | Azure Batch canary setup can proceed. |
| `Microsoft.ContainerRegistry` | `Registered` | Azure Container Registry setup can proceed. |
| `Microsoft.ContainerInstance` | `Registered` | ACI smoke setup can proceed if needed. |
| `Microsoft.Storage` | `Registered` | Storage account/container setup can proceed. |

This means the account is useful for planning, template validation, and Azure
resource setup, but it does not yet satisfy Track 39 or Track 43 Azure runtime
evidence. Runtime evidence still requires disposable resources and a completed
canary job.

## Candidate next steps

Before recording Azure Batch runtime acceptance, complete these actions in the
subscription:

1. Create or identify a test resource group that is approved for KairoECS
   canary resources.
2. Create a small Azure Batch account, pool, storage account, and container
   image source.
3. Run the smallest factory-bottleneck canary through
   `cloud/azure/submit-experiment.ps1`.
4. Record the Batch account, pool id, job id, task id, final status, output URI,
   and checksum evidence in `docs/cloud-hpc/runtime-evidence-boundary.md`.

Provider registration and resource creation are subscription mutations and may
incur quota or cost consequences. They should only be performed intentionally
against a disposable canary resource group.

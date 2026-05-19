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
| `Microsoft.Batch` | `NotRegistered` | Azure Batch canary jobs cannot run yet. |
| `Microsoft.ContainerRegistry` | `NotRegistered` | Azure Container Registry publication cannot run yet. |
| `Microsoft.ContainerInstance` | `NotRegistered` | ACI smoke runs cannot run yet. |

This means the account is useful for planning, template validation, and provider
registration follow-up, but it does not yet satisfy Track 39 or Track 43 Azure
runtime evidence.

## Candidate next steps

Before recording Azure Batch runtime acceptance, complete these actions in the
subscription:

1. Register the required providers: `Microsoft.Batch`,
   `Microsoft.ContainerRegistry`, `Microsoft.ContainerInstance`, and
   `Microsoft.Storage`.
2. Create or identify a test resource group that is approved for KairoECS
   canary resources.
3. Create a small Azure Batch account, pool, storage account, and container
   image source.
4. Run the smallest factory-bottleneck canary through
   `cloud/azure/submit-experiment.ps1`.
5. Record the Batch account, pool id, job id, task id, final status, output URI,
   and checksum evidence in `docs/cloud-hpc/runtime-evidence-boundary.md`.

Provider registration and resource creation are subscription mutations and may
incur quota or cost consequences. They should only be performed intentionally
against a disposable canary resource group.

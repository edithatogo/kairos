# Azure Batch Canary Evidence: 2026-05-20

This file records the first live Azure Batch canary run for Track 39/43. It is
runtime substrate evidence only. It proves that the student Azure subscription
can create a disposable Batch account, allocate a low-priority CPU node, run a
task, and return output. It does not prove KairoECS container execution, GPU
parity, HPC scaling, or production registry acceptance.

## Azure Context

| Field | Value |
|---|---|
| Subscription | `Azure for Students` |
| Account | student Azure account |
| Region | `australiaeast` |
| Resource group | `rg-kairos-batch-canary-20260520` |
| Storage account | `kairoecs0520damo` |
| Blob container | `kairos-output` |
| Batch account | `kairobatch0520damo` |
| Batch endpoint | `kairobatch0520damo.australiaeast.batch.azure.com` |

Account-specific subscription and tenant identifiers are intentionally omitted
from this public evidence note.

## Pool Evidence

| Field | Value |
|---|---|
| Pool id | `kairoscanarypool` |
| VM size | `standard_a1_v2` |
| Dedicated nodes | `0` |
| Low-priority nodes | `1` for the canary, then resized back to `0` |
| Image publisher | `canonical` |
| Image offer | `0001-com-ubuntu-server-jammy` |
| Image sku | `22_04-lts` |
| Node agent SKU | `batch.node.ubuntu 22.04` |
| Allocation result | `steady` with `currentLowPriorityNodes: 1` before task execution |

The Batch account reported CPU-family quota for small CPU VM families and zero
quota for GPU/HPC families such as NC, NV, H, HB, and ND. Therefore this pass is
not GPU or HPC hardware evidence.

## Task Evidence

| Field | Value |
|---|---|
| Job id | `kairos-canary-20260520` |
| Task id | `kairos-canary-task-001` |
| Task state | `completed` |
| Result | `success` |
| Exit code | `0` |
| Start time | `2026-05-20T00:01:05.127121Z` |
| End time | `2026-05-20T00:01:05.228903Z` |
| Retry count | `0` |

Command line:

```bash
/bin/bash -lc 'echo KAIRO_AZURE_BATCH_CANARY; date -u; uname -a; python3 --version || true; echo resource_group=rg-kairos-batch-canary-20260520; echo pool=kairoscanarypool; echo job=kairos-canary-20260520; echo task=kairos-canary-task-001'
```

Stdout:

```text
KAIRO_AZURE_BATCH_CANARY
Wed May 20 00:01:05 UTC 2026
Linux ed781a2390b249e1972ee2943e17c902000000 6.8.0-1052-azure #58~22.04.1-Ubuntu SMP Thu Mar 26 05:02:21 UTC 2026 x86_64 x86_64 x86_64 GNU/Linux
Python 3.10.12
resource_group=rg-kairos-batch-canary-20260520
pool=kairoscanarypool
job=kairos-canary-20260520
task=kairos-canary-task-001
```

Stderr was empty.

## Remaining Azure Gaps

- KairoECS CLI/container execution was not run because the local Docker CLI
  could not connect to a Docker daemon, so the repo image could not be built and
  pushed from this host.
- The `cloud/azure/submit-experiment.ps1` path has not yet executed against a
  readable `kairo-ecs-cli` container image.
- Factory-bottleneck scenario output/checksum evidence was not produced from
  KairoECS.
- GPU or HPC hardware execution was blocked in this subscription because the
  Batch quota report showed zero GPU/HPC-family quota.

## Cleanup

The compute pool was resized back to zero nodes after the canary:

```powershell
az batch pool resize --pool-id kairoscanarypool --target-dedicated-nodes 0 --target-low-priority-nodes 0
```

To remove all disposable canary resources after evidence review:

```powershell
az group delete --name rg-kairos-batch-canary-20260520 --yes --no-wait
```

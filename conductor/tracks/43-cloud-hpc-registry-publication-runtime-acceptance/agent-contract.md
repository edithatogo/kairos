# 43 Cloud/HPC Registry Publication & Runtime Acceptance - agent-contract.md

## hpc-registry-agent

Owns the HPC registry manifest, publication workflow, and validation gate.

## cloud-agent

Owns Docker, Kubernetes, Slurm, and provider runtime evidence. Must not mark runtime claims complete using offline validation alone.

## release-agent

Owns publication approval and release notes.

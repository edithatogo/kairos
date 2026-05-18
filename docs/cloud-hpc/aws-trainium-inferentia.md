# AWS Trainium and Inferentia

AWS Trainium and Inferentia are AWS accelerator families for model training and
inference. They are relevant here as a future specialized-compute route, not as
a free always-on path.

## What to use them for

- client/runtime smoke for Neuron-enabled environments
- validation of AWS-specific accelerator integration code
- very small prototype runs when trial credits or approved access are available

## What they do not give you for free

- stable always-on accelerator capacity
- general-purpose GPU parity evidence for Track 32
- TPU-like notebook access

## Current repository boundary

There is no repo-local Trainium/Inferentia harness yet. The current cloud/HPC
surface is limited to offline validation docs and provider-agnostic batch
manifests. When an AWS-specific smoke harness is added, it should live beside the
other cloud guidance and keep the actual runtime proof separate from offline
manifest checks.

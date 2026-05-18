# Specialized Compute Options

This page collects the closest low-cost or trial-backed ways to exercise
specialized compute outside the repo.

## Summary

| Provider | Specialized compute | Low-cost access path | Best use | What it does not replace |
|---|---|---|---|---|
| Google Cloud | Cloud TPU | Free trial credits, Free Tier products, or TPU Research Cloud for eligible researchers/students | TPU smoke tests and TPU-specific prototype notebooks | Stable CI and guaranteed TPU capacity |
| AWS | Trainium and Inferentia | AWS Free Tier credits for new accounts, plus public training/tutorial content; actual accelerator use consumes credits | Trainium/Inferentia client/runtime smoke and Neuron compatibility checks | Free always-on accelerator capacity |
| Azure | GPU VMs, Azure Batch, Azure Machine Learning compute | Azure for Students / free account credits; batch/ML compute can be spun up inside credit limits | GPU or HPC smoke on trial credits, especially Batch or AML jobs | Free GPU/accelerator capacity beyond the trial/credit limit |
| GitHub | macOS runners, GPU larger runners, self-hosted runners | Standard GitHub-hosted runners are free with the repo; GPU larger runners require Team/Enterprise access | Metal-adjacent smoke, standard CI, or GPU if the org has larger-runner access | TPUs and general HPC clusters |
| NVIDIA NIM | Hosted NVIDIA-GPU inference service | Use the configured endpoint or a free/partner plan if your account has one | NVIDIA library/runtime compatibility smoke | GPU kernel parity and benchmark evidence |

## Practical recommendation

1. Use GitHub macOS runners for Metal-adjacent browser and native smoke.
2. Use your local M1 MacBook Pro for repeatable Apple Metal follow-up.
3. Use Colab TPU for a quick TPU notebook smoke if the goal is only to prove
   TPU access and a tiny tensor op.
4. Use NIM when you want NVIDIA-GPU-backed client/runtime smoke without setting
   up your own box.
5. Use cloud credits on AWS, Azure, or Google Cloud only if you need a small
   provider-specific smoke that is not covered by the free routes above.

## AWS Trainium and Inferentia route

See [AWS Trainium and Inferentia](aws-trainium-inferentia.md) for the current
roadmap position and the current repo boundary. That route is roadmap-backed,
not yet executable from the checked-in tree.

## Cost boundary

The usual free-tier reality is:

- Google Cloud TPU has free-credit and research pathways, but not an always-free
  TPU pool.
- AWS Trainium and Inferentia are accelerator products, not free-tier hardware.
- Azure gives student/free-account credits and free monthly services, but GPU
  or ML compute still consumes credit.

If the smoke is truly only a few minutes, the trial credits can often cover it,
but that is still credit-backed usage rather than an always-free accelerator.

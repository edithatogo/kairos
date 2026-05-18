# Free Testing Routes

This document records the lowest-cost validation paths for the GPU-adjacent
tracks in this repository. It does not replace the hardware-backed evidence
required for release claims.

## What each route is good for

| Route | Best used for | What it does not prove |
|---|---|---|
| GitHub-hosted macOS runner | Metal-backed compile/smoke validation for browser or native GPU surfaces | Performance claims, long-running parity, or local device-specific throughput |
| Local Apple Silicon MacBook Pro | Repeatable Metal smoke tests on real Apple hardware | Cross-platform throughput or non-Apple backend behavior |
| Google Colab free GPU runtime | Ad hoc GPU notebook smoke tests on the available NVIDIA GPU backend | Stable CI, guaranteed availability, or current Rust GPU track validation |
| NVIDIA NIM endpoint or container | NVIDIA-GPU-backed library smoke tests, client/runtime compatibility checks, CUDA-visible dependency validation | GPU kernel parity, benchmark claims, or browser WebGPU behavior |
| Google Colab free TPU runtime | Ad hoc TPU notebook smoke tests for TPU-specific prototypes | Stable CI, guaranteed availability, or direct validation of the current GPU tracks |
| No free general-purpose route | ASIC-style validation | Anything that needs vendor-specific ASIC hardware or a provider-managed accelerator |

## Recommended use in this repo

1. Use GitHub macOS runners first for Track 33-style Metal-adjacent smoke checks.
2. Use the M1 MacBook Pro for local follow-up runs when device-specific Metal
   behavior matters.
3. Use Colab GPU for quick NVIDIA GPU notebook smoke when only a free T4-style
   runtime is available.
4. Use NVIDIA NIM for Track 32-style NVIDIA library smoke checks when an
   endpoint or container is available.
5. Use Colab TPU only for the dedicated notebook in `notebooks/colab_tpu_dedicated_smoke.ipynb`;
   the current repository does not have a TPU backend.

## Boundary notes

- NIM is a GPU-backed inference service, not a general GPU benchmark farm.
- Colab GPU is free to use, but resources are not guaranteed and the available
  GPU type can vary over time.
- Colab is free to use, but resources are not guaranteed and the available TPU
  types can vary over time.
- TPU and ASIC support are not currently incorporated into the repository
  implementation surfaces. They remain future track candidates unless a track
  explicitly adds them.
- For provider-specific accelerators and trial-credit-backed smoke, see
  `docs/cloud-hpc/specialized-compute-options.md`.

## Runnable entry points

- GitHub macOS smoke: `.github/workflows/gpu-free-smoke.yml`
- Colab GPU notebook: `notebooks/colab_gpu_smoke.ipynb`
- NIM smoke: `node scripts/nim/nim-gpu-smoke.mjs`
- Colab TPU notebook: `notebooks/colab_tpu_dedicated_smoke.ipynb`

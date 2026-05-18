# Hardware Requirements

No performance claim is accepted until a hardware-backed validation run records:

| Backend | Minimum target | Required evidence |
|---|---|---|
| wgpu Vulkan | Discrete GPU with current Vulkan driver | Build log, adapter name, parity output |
| wgpu Metal | Apple Silicon or supported AMD Mac | Build log, adapter name, parity output |
| wgpu DX12 | Windows discrete GPU | Build log, adapter name, parity output |
| CUDA | NVIDIA GPU with supported CUDA toolkit | Build log, GPU model, parity output, benchmark output |

CPU-only CI may run compile and fallback parity tests but must not publish speedup numbers.

## Lowest-cost validation routes

Use the routes in `free-testing-routes.md` when you need smoke coverage without
specialized hardware spend. These are not substitutes for the acceptance
evidence above, but they are the practical path for early validation:

- GitHub-hosted macOS runners for Metal-adjacent compile and smoke checks.
- A local Apple Silicon MacBook Pro for repeatable Metal device smoke.
- Colab free GPU runtimes for the available NVIDIA GPU notebook smoke route.
- NVIDIA NIM for NVIDIA-GPU-backed library smoke and runtime compatibility
  checks when an endpoint or container is available.
- Google Colab free TPU runtimes for TPU-specific prototype notebooks.

TPU and ASIC support are not currently incorporated into the repository
implementation surfaces, so they remain future work unless a track adds them.

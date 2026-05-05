# Hardware Requirements

No performance claim is accepted until a hardware-backed validation run records:

| Backend | Minimum target | Required evidence |
|---|---|---|
| wgpu Vulkan | Discrete GPU with current Vulkan driver | Build log, adapter name, parity output |
| wgpu Metal | Apple Silicon or supported AMD Mac | Build log, adapter name, parity output |
| wgpu DX12 | Windows discrete GPU | Build log, adapter name, parity output |
| CUDA | NVIDIA GPU with supported CUDA toolkit | Build log, GPU model, parity output, benchmark output |

CPU-only CI may run compile and fallback parity tests but must not publish speedup numbers.

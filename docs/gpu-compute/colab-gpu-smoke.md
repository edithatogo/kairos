# Colab GPU Smoke

Maturity: experimental smoke route. This is useful for ad hoc free GPU access
checks, not release evidence.

This notebook is the free Colab GPU smoke route for the available NVIDIA GPU
runtime in this session, including T4.

## Notebook

- [notebooks/colab_gpu_smoke.ipynb](../../notebooks/colab_gpu_smoke.ipynb)

## Reproducibility command

1. Open the notebook in Google Colab.
2. Change the runtime type to GPU.
3. From the Colab menu, run `Runtime > Run all`.
4. Record the runtime type, GPU name, JAX backend, JAX device list, and matmul
   checksum from the completed notebook output.

Equivalent cell commands:

```bash
nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader
```

```python
import jax
import jax.numpy as jnp

print({
    "backend": jax.default_backend(),
    "jax_version": jax.__version__,
    "devices": [str(device) for device in jax.devices()],
})
assert any("GPU" in str(device).upper() for device in jax.devices())

@jax.jit
def tiny_matmul(x):
    return jnp.matmul(x, x) + 1.0

print(float(tiny_matmul(jnp.ones((32, 32), dtype=jnp.float32))[0, 0]))
```

## Expected output

A passing smoke run shows:

- `nvidia-smi` prints one NVIDIA GPU row, commonly a T4 on the free tier.
- `jax.default_backend()` is `gpu`.
- At least one `jax.devices()` entry contains `GpuDevice`.
- The tiny matmul checksum prints `33.0`.

## Boundary

This smoke checks GPU access and a minimal tensor operation. It does not
validate the current Rust GPU tracks, and it does not prove production GPU
throughput. Track 32 budget and DES event-transfer checks remain validated by
the Rust tests for `crates/kairo-ecs-gpu`.

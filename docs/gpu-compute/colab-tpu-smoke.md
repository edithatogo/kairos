# Colab TPU Smoke

This notebook is the free TPU smoke route for future TPU-specific prototype
work.

## Notebook

- [notebooks/colab_tpu_smoke.ipynb](../../notebooks/colab_tpu_smoke.ipynb)

## Run

1. Open the notebook in Google Colab.
2. Change the runtime type to TPU.
3. Run the device listing cell.
4. Run the tiny JAX matmul cell.

## Boundary

This smoke checks TPU access and a minimal tensor operation. It does not
validate the current Rust GPU tracks, and it does not prove production TPU
throughput.

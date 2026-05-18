# NVIDIA NIM Smoke

This directory holds the optional NIM-backed GPU smoke for the repo.

## Contract

The smoke script expects an OpenAI-compatible NIM endpoint:

- `NVIDIA_NIM_BASE_URL` or `NIM_BASE_URL`
- `NVIDIA_NIM_API_KEY` or `NIM_API_KEY`
- optional `NVIDIA_NIM_MODEL` or `NIM_MODEL`

## Behavior

- If the required endpoint variables are missing, the script exits `0` with a
  `status: skipped` message.
- If they are present, the script calls `/v1/models` and
  `/v1/chat/completions`, then prints a small JSON receipt.

## CI strictness

Trusted GitHub Actions runs require both `NVIDIA_NIM_BASE_URL` and
`NVIDIA_NIM_API_KEY`. Forked pull requests can still skip the smoke because
GitHub withholds secrets there.

## Intended use

Use this for NVIDIA-GPU-backed client/runtime compatibility smoke, not for
kernel parity or benchmark claims.

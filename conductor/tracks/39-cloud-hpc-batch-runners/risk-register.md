# Risk Register: Track 39 Cloud / HPC Batch Runners

Severity scale: Likelihood 1-5 x Impact 1-5. Low 1-4, Medium 5-9, High 10-16, Critical 17-25.

| Risk | Likelihood | Impact | Severity | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Container image size exceeds reasonable pull/transfer limits (>2 GB) | 4 | 3 | 12 | Use multi-stage Docker builds, distroless or Alpine base image, strip debug symbols, exclude build toolchain from final layer; measure image size in cloud-smoke CI | cloud-agent | Docker image exceeds 1.5 GB in CI build |
| Spot instance interruption causes data loss (checkpoint not written before termination) | 3 | 5 | 15 | Trap SIGTERM in entrypoint with 90-second checkpoint window; write checkpoint atomically (write to temp file, then rename); validate checkpoint integrity with checksum before resuming; test spot interruption locally with simulated SIGTERM | cloud-agent | Checkpoint integrity test fails or checkpoint file is truncated |
| Cloud provider API drift breaks job definitions between releases | 3 | 3 | 9 | Pin provider API versions in job definitions; validate against provider schema in CI using dry-run/submit-and-cancel; document minimum provider API versions in `docs/cloud-hpc/`; test against provider emulators where available (LocalStack, Azurite, GCP emulator) | cloud-agent | Schema validation fails on previously passing job definition |
| Kubernetes operator version compatibility (API deprecations across K8s 1.26-1.31) | 2 | 4 | 8 | Target Kubernetes `batch/v1` (GA since 1.21) for Job API; use `apiextensions.k8s.io/v1` for CRD; test operator against 3 latest minor K8s versions in CI matrix; avoid beta API surface | cloud-agent | Operator smoke test fails on a supported K8s version |
| Multi-arch Docker build increases CI runtime and cost (2x build time per commit) | 3 | 2 | 6 | Only build multi-arch on release tags; PR-level `cloud-smoke.yml` builds native arch only; use Docker Buildx cache with registry backend; gate full multi-arch build behind release pipeline trigger | cloud-agent | Cloud-smoke CI runtime exceeds 30 minutes |

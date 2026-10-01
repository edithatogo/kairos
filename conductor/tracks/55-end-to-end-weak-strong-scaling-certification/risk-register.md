# Track 55 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Downstream track evidence is incomplete | Certification blocked | Keep Track 55 non-Done until dependencies close |
| Scaling results are not reproducible | Invalid parity claim | Require raw manifests, checksums, and environment metadata |
| Comparison baselines are unfair | Misleading SOTA claim | Record workload differences and omit unfair comparisons |
| Integrated scenario is too narrow | Overbroad release claim | Cover PDES, distributed, GPU, I/O, FMI, and restart paths |
| CI passes but live HPC fails | False readiness | Treat live HPC evidence as required for Done |

# 43 Cloud/HPC Registry Publication & Runtime Acceptance - risk-register.md

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10), with residual gating status tracked in the track plan.

| Risk | Impact | Mitigation |
|---|---|---|
| Offline scaffold mistaken for runtime proof | False production-readiness claim | Validator requires pending live evidence markers |
| Unsigned image or mutable tag | Supply-chain ambiguity | Require digest, SBOM, and signature/attestation before promotion |
| Cloud credentials leak | Account compromise | Protected environment and least-privilege cloud roles |
| Scheduler canary unavailable | Track remains blocked | Keep partial-scoped status and evidence boundary |
| Health below 9.5 | Release quality gap | Track 44 blocks production publication |

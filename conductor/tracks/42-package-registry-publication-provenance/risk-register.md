# 42 Package Registry Publication & Provenance - risk-register.md

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10), with residual gating status tracked in the track plan.

| Risk | Impact | Mitigation |
|---|---|---|
| Accidental public write | Irreversible registry release | Default `publish=false`, protected environment, release-manager approval |
| Long-lived token compromise | Supply-chain compromise | Prefer OIDC/trusted publishing; token fallback must be scoped and rotated |
| Package name collision | Failed release or confusing package | Registry-name check and fallback before production publish |
| Overclaiming preview packages | User trust and compatibility risk | Compatibility notes and preview labels required |
| Missing provenance | Lower supply-chain trust | Provenance/attestation gate blocks production publication |
| Health score below 9.5 | Release quality gap | Track 44 blocks production registry writes |

# Track 69 Risk Register

Severity scale: Low (1), Medium (2), High (3), Critical (4), Release Blocker (5).

| Risk | Likelihood | Impact | Severity | Mitigation |
|---|---|---|---|---|
| Inverting non-closed-form cumulative hazard functions causes numerical instability | Medium (2) | High (3) | High (3) | Use bracketed Brent-Dekker root finding with strict convergence tolerance |
| Uncalibrated hazard rates produce unrealistic explosion of zero-time transitions | Low (1) | Critical (4) | High (3) | Allow valid zero hazards; validate model units and event accumulation, reject unsupported explosive models explicitly, and never silently clamp durations because it changes the target process |
| Substream collision perturbs deterministic replay | Low (1) | Critical (4) | High (3) | Derive dedicated hazard substream keys from entity ID and stage counter |

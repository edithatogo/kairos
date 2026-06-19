# HPC Scaling Certification

Track 55 is the final evidence gate for production HPC parity claims. The
current repository state only defines the scenario and manifest contract; it
does not certify weak scaling, strong scaling, or end-to-end HPC parity.

Certification requires both weak and strong scaling profiles with raw result
references, checksums, hardware metadata, scheduler metadata, toolchain
metadata, and review signoff. Draft manifests may record active blockers while
Tracks 47-54 are incomplete, but they must not use certified language.

The canonical draft inputs are:

- `conductor/tracks/55-end-to-end-weak-strong-scaling-certification/scenarios.json`
- `conductor/tracks/55-end-to-end-weak-strong-scaling-certification/evidence.json`
- `scripts/validation/validate-hpc-scaling-certification.mjs`

Release claims stay bounded to planned, scaffolded, preview, or
evidence-limited language until this track closes with live raw results.

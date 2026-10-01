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
- `conductor/hpc-evidence/manifests/track55-local-scaling-smoke-scaffold.json`
- `conductor/hpc-evidence/manifests/track55-live-weak-scaling-template.json`
- `conductor/hpc-evidence/manifests/track55-live-strong-scaling-template.json`
- `scripts/validation/validate-hpc-scaling-certification.mjs`

The local scaffold records only the repository-side smoke and contract gates.
It is useful for review, but it is not live hardware evidence and cannot be
used to mark either scaling profile as certified. The live weak and strong
templates must be copied into real `live-hpc` manifests with a pushed 40
character commit SHA, immutable raw artifact reference, sha256 checksum,
hardware, scheduler, storage, toolchain, reviewer, and `waiver.status: none`
before any certification language is allowed.

Release claims stay bounded to planned, scaffolded, preview, or
evidence-limited language until this track closes with live raw results.

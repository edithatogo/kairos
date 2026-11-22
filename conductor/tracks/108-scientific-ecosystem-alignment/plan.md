# Plan

- [x] Reconcile current source, active ownership and reusable evidence.
- [x] Reconcile Track 107 and the accepted installed source baseline.
- [x] Qualify minimal core and optional Arrow/Polars features without changing simulation semantics.
- [x] Verify RNG replay, schema and buffer ownership against CareOps and Innovate consumers.
- [x] Exercise argument rejection, interrupted output and independent retained-buffer controls.
- [x] Run focused native and installed-consumer checks with exact artifact receipts.
- [x] Review independently, apply fixes and run the provider repository and hosted gates.
- [x] Record integrator acceptance for the exact installed candidate.

Ownership: existing Kairos integrator retains shared records and pins. No native operation is taken over.

## Phase closeout gate

Before completing a phase, run `$conductor-review` using agents and Auto-apply accepted review fixes within owned paths. Record blocked or cross-track findings. Reconcile `conductor/tracks.yaml`, `conductor/tracks.md`, `conductor/phase-closeout.yaml` and `conductor/status.md` through their integrator. Run `validate_conductor_phase_gates.ps1` and the test matrix. Commit and push the cleaned slice, then run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`. Record the next-phase decision and advance only after accepted evidence.

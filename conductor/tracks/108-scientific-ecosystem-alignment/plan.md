# Plan

- [ ] Reconcile current source, active ownership and reusable evidence.
- [ ] Reconcile Track 107 and accepted source baseline with integrator before pin changes.
- [ ] Qualify minimal core and optional Arrow/Polars features without changing simulation semantics.
- [ ] Verify RNG replay, schema and buffer ownership against CareOps and Innovate consumers.
- [ ] Write failing regression cases before implementation changes.
- [ ] Run focused native and installed-consumer checks with resource receipts.
- [ ] Agent review, apply fixes and run required repository/hosted gates.
- [ ] Integrator acceptance and archive only after all native gates pass.

Ownership: existing Kairos integrator retains shared records and pins. No native operation is taken over.

## Phase closeout gate

Before completing a phase, run `$conductor-review` using agents and Auto-apply accepted review fixes within owned paths. Record blocked or cross-track findings. Reconcile `conductor/tracks.yaml`, `conductor/tracks.md`, `conductor/phase-closeout.yaml` and `conductor/status.md` through their integrator. Run `validate_conductor_phase_gates.ps1` and the test matrix. Commit and push the cleaned slice, then run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`. Record the next-phase decision and advance only after accepted evidence.

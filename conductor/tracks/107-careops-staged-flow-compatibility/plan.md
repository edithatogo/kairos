# Track 107 plan

1. Review contract and API boundary with reviewer agent.
2. Implement deterministic scheduler-backed staged resource runtime and contract fixture/test.
3. Run focused and milestone checks; capture exact command, commit/toolchain/input hashes and outputs.
4. Run conductor review; apply accepted in-scope fixes and synchronize registry/status surfaces.
5. Handoff candidate to CareOps R10.3; parent decides consumer migration and pin qualification.

No phase may claim complete from planned checks. Parent pin remains unchanged until CareOps acceptance.

## Phase closeout gate

Before advancing, run `$conductor-review`, Auto-apply accepted review fixes, then run `validate_conductor_phase_gates.ps1` and the owned checks in `test-matrix.md`. Synchronize `conductor/tracks.yaml`, `conductor/tracks.md`, `conductor/phase-closeout.yaml`, and `conductor/status.md`. Commit and push the cleaned slice, record its SHA/ref, and run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`. Until then the phase remains pending and the next-phase decision is hold.

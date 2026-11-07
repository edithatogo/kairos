# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Retirement | Every old host-engine constructor fails before state/time mutation, including with a fallback environment variable set. | Pending for this track |
| Native execution | Build the Rust library/Wasm artefact; installed test schedules/cancels/runs and verifies native event counts and error paths; missing artefact fails the native lane. | Pending for this track |
| Lifetime | Double-close, use-after-close, invalid handles, ABI mismatch, buffer ownership and overflow cases reject. | Pending for this track |
| Replay | Actual native stream generation and event traces match versioned fixtures; reading expected-vector length is insufficient. | Pending for this track |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.


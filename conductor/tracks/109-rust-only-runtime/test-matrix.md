# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Retirement | Every old host-engine constructor fails before state/time mutation, including with a fallback environment variable set. | Phase 0 accepted after the [implementation and retirement checks](../../frontier-implementation-20261107.md) and [exact candidate binding CI](https://github.com/edithatogo/kairos/actions/runs/45051171197). |
| Native execution | Build the Rust library/Wasm artefact; installed test schedules/cancels/runs and verifies native event counts and error paths; missing artefact fails the native lane. | [Installed native-core-v1](https://github.com/edithatogo/kairos/actions/runs/45051171243) passed on both profiled targets. Other installed adapters retain their own gates. |
| Lifetime | Double-close, use-after-close, invalid handles, ABI mismatch, buffer ownership and overflow cases reject. | Accepted for C ABI v1 and the profiled Python adapter. The downloaded macOS archives repeated the C and Python checks. |
| Replay | Actual native stream generation and event traces match versioned fixtures; reading expected-vector length is insufficient. | Candidate fixtures and the [alpha replay disposition](../../release/beta-scope.md#alpha-replay-compatibility) bind the RNG and event semantics. |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.


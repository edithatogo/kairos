# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Semantic identity | Serial and parallel executions consume the same event tapes and satisfy exact/tolerance-based contracts. | Pending for this track |
| Overhead | Thread creation counts and profiler traces show whether the targeted overhead was actually removed. | Pending for this track |
| Performance | Raw repeated wall-clock/RSS samples from named hardware and equivalent work; useful events exclude rolled-back work. | Pending for this track |
| Regression | Dedicated comparable runners evaluate declared thresholds; noisy shared-runner smoke is not performance certification. | Pending for this track |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.


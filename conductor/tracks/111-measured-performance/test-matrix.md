# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Semantic identity | Same tape and ordered per-LP state, with `production_parity` and `production_protocol` on both executors. | Matched ring and conservative semantics passed locally. |
| Overhead | Thread identity across two horizons and a native stack sample of the original scoped runtime. | 64 distinct callback threads before the change and one with the pool. Broader profiling remains open. |
| Performance | Raw paired wall-clock and process RSS from the matched ring on named hardware. | Five event costs and eight historical cases recorded on Apple M1 Max. |
| Regression | Dedicated comparable runners evaluate declared thresholds; noisy shared-runner smoke is not performance certification. | Pending for this track |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.


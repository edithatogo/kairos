# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| S01–S04 | Receiving solvers and game oracles | See the Paignia source dispositions in the track plan |
| S05–S08 | Independent vectors, statistical fixtures and analytical hazard timing | Bounded implementations and measured regression results in the scientific register |
| S09–S10 | Native nonidentity outputs, scalar/batch parity, installed host checks and lifetime regressions | Affine surrogate and experimental shared queue. ONNX, TensorRT and Burn are unavailable |
| S11–S17 | Actual devices/transports/external FMUs/filesystems or explicit capability-unavailable result; no mock or fixture metadata accepted as native proof. | Pending for this track |
| Scientific claims | Every supported method has a measured residual/error criterion and held-out test; classify each unsupported method explicitly. | Pending for this track |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.


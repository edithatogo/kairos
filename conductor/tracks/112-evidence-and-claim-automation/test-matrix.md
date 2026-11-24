# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Integrity | Verify retained result digests and exact run identity. | Changed Arrow bytes, benchmark bytes and rehashed parity/source mismatches reject. |
| False claims | Require the matching evidence class and capability, including native target. | Synthetic promotion, unrelated native evidence, wrong target, borrowed empirical evidence and unsupported frontier promotion reject. |
| Drift | Check generated Markdown, evidence paths and completion tracks. | A stale table exits 1, regeneration restores a clean pass and dangling references reject. |
| Scope | Preserve target, execution and exact source provenance on qualification receipts. | The register retains the limits on historical receipts and links the current native tests and raw output. |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | 27 accepted entries and 20 passing positive and negative tests |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.


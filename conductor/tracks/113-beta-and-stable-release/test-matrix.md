# Test matrix

| Gate | Required evidence | Current acceptance |
|---|---|---|
| Beta | Every enumerated required gate has candidate-specific evidence or the capability is excluded. | [Engineering and maintainer decisions accepted](handoff.md#native-core-v1-decision--25-november-2026) for `e2875387`; shared phase-ledger integration and publication are separate. |
| Installed packages | [Native beta installed artifacts workflow](../../../.github/workflows/native-beta.yml) executes the Rust, C and Python artifacts on Linux x86-64 and macOS ARM64. | Both selected platform receipts passed at `e2875387`. |
| Installed CareOps | The pinned public CareOps lease-contention case runs against the same installed SDK from both legs of the matrix. | Passed with the same shared SDK digest on both targets. |
| Candidate provenance | [Release Attestations](../../../.github/workflows/release-attestations.yml) verifies the selected installed run, both receipt digests, and all six package paths before scanning the contents and signing the SPDX SBOM. | The downloaded bundle and signed SBOM were verified against all five distinct tested digests. |
| Stable | Independent reproduction, upgrade/replay fixtures, supported limits and no unresolved critical/high defects in supported scope. | Pending for this track |
| Publication | Actual tag/registry/provenance readback is distinct from CI or PR success. | Pending for this track |
| Capability/claim consistency | node scripts/validation/validate-frontier-evidence.mjs and node --test tests/conformance/frontier-evidence-check.mjs | Structural check only; no new runtime claim |
| Phase registry | pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1 | Required before phase closure |
| Clean source publication | pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree | Required after the reviewed pushed commit |

Use the real native/scientific commands selected by the phase owner from existing Cargo targets and independent references. Missing hardware/data/toolchains are recorded as unavailable, never a passing simulated result. Test selection must exercise the advertised behaviour and invalid-input paths. Capture command/cwd/revision/feature set, toolchain/environment, input/seed/hash, output hash and exit status. A receipt from another revision is historical until impact is assessed.


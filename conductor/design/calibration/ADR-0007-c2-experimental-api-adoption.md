# ADR-0007 — C2 experimental production API adoption

Status: proposed for independent review; no export or runtime acceptance.
Date: 2026-10-05. Owners: Track03/21/01/22 and API/MSRV Track25/30.

## Need

The optional production calibration Flow bridge must use engine-owned fidelity
and route types. Test-only path imports cannot establish a production bridge.
Existing fidelity remains private; C2.0 freezes test definitions only. This ADR
records the required adoption gate before C2.2/C2.3 exports are implemented.

## Proposed API form

- DES exposes an experimental `fidelity` module containing the existing policy,
  mode, scope, decision, errors and adapter, plus the frozen admission permit.
  Existing Flow types retain their current exports. The new planned hook and
  typed domain-control ingress are additive FlowRuntime methods; existing mutable
  hooks and callback causes retain their behavior.
- ABM exposes an experimental `spatial` module containing the frozen graph,
  route, movement profile, transit context/progress/phase and typed errors.
  Integer units, tie ordering and scheduled controls follow the frozen contract.
- Calibration keeps intrinsic provider, stream keys and bridge states crate
  private. Its optional `flow` feature depends on production DES/ABM crates;
  no DES dependency on calibration and no alternate test-only implementation.
- No stable API baseline, portable checkpoint format or clinical validation is
  established by these experimental exports. Track25/D4 owns release qualification.

## Required review and conformance evidence

Before an export is accepted, record independent objections and dispositions,
exact exported names/signatures and compatibility against existing callers.
Run existing default DES/ABM tests and Rust1.76 checks, optional calibration and
Flow-feature Rust1.88 checks, canonical Rust1.99 tests/clippy/fmt, and actual
production-feature compile tests. Preserve all existing scheduler/RNG goldens.

C2.0 fixtures must graduate to executable native tests as implementations land.
All required named tests execute; missing API reds and ignored tests cannot close
C2.1. Hook rejection tests preserve command/context atomicity while acknowledging
consumed source events and advanced dispatch accounting. Actual provider/bridge/
carrier tests prove paired outcomes and owned Service continuation; complete
runner checkpoint/rebinding remains the separate Track22 gate.

No source export is authorized by this proposal alone. Independent review and
coordinator acceptance bind a future implementation packet to these requirements.
Cargo/lock changes require their own exclusive reservation and version review.
Native hosted qualification and reviewed parent pin publication remain mandatory.

## Source authority

Frozen definitions: `c20-work-provider-v1.md`, `c20-admission-bridge-v1.md`,
`c20-transit-execution-v1.md`; existing private semantics:
`c2-execution-admission-v1.md` and `ADR-0006-flow-runtime-identity.md`.
No contract used by an active worker is changed by this proposal.

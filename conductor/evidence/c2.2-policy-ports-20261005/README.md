# C2.2 policy test-port local qualification

Mode commit ac56c999 and lineage commit51128b62 move the original private
source-copy integration fixtures into actual in-crate unit modules. Independent
readback proves every helper and test tail byte-identical to the prior fixtures.
Only header imports and crate-root cfg(test) wiring change. The existing fidelity
implementation, TSV, manifests and lockfile remain unchanged.

Saved raw logs and worker receipts retain actual argv, cwd, source and hashes.
Rust1.76 and1.99 each execute all10mode and3lineage cases once; strict canonical
clippy passes. Root formatting and diff checks pass. The existing Q5.2 explicit
measurement test remains ignored by ordinary package testing; it is not a skipped
fidelity case. Hosted exact-head CI and publication remain open.

The additive API decision is separately reviewed and integrated. Fidelity remains
private at this head; production Adapter remains non-Clone. Export smoke and
borrowing admission permit are subsequent gated work. No full C2.2, paired C2.1,
portable checkpoint, clinical, ED MVP or stable-release acceptance is claimed.

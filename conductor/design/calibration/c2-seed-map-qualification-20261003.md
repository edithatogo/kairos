# C2 seed-map foundation qualification — partial

Reviewed normative seed-map.v1 bytes/digest/seed/draws against independent Python
goldens. Purpose streams use the unchanged engine SplitMix recurrence. Exact UTF-8
length framing avoids ambiguous concatenation; no normalization or thread-derived
identity. Finite registry rejects distinct identities with the same u64 seed and
preserves the original entry. No global collision-free claim.

Actual Rust1.98.1 and actual Rust1.88.0 explicit RUSTC/RUSTDOC/PATH package tests:
13 passed (8 unit,5 integration), zero doc tests. Both include byte-boundary1024/
1025, per-field/root-seed changes, collision preservation and continuation at draw
positions0/1. An in-progress UTF-8 boundary test failed and was corrected before
final qualification; original log is retained. Exact1.88 rustc6b00bc3882025-06-23
confirmed by target/.rustc_info.json. All cargo-deny categories passed0.20.2 with
existing allowlist/bans/advisories/sources intact; no engine RNG source change.
Logs: parent .artifacts/blocker-resolution/c2-seed-actual-1.88-final.log and
c2-seed-policy.log; worker .artifacts/c2-seed/result.json in isolated seed checkout.

Reordered registration/interleaved draws are deterministic; this does not establish
actual worker-count execution. Repeated identity registration returns a fresh stream
at position0; one advancing stream per purpose remains orchestration responsibility.
Snapshots can branch; they are opaque owned in-memory continuation, not a portable
checkpoint codec. C2 mode/duration/transit/fidelity integration remains unimplemented
and C2 parent tasks remain unchecked.

Owner CI now includes calibration and Arrow on native Linux/macOS Rust1.98.1, plus
separate targeted calibration Rust1.88 floor tests. Child compiler paths/host triples
are explicit. Exact hosted qualification is required before updating the parent pin.

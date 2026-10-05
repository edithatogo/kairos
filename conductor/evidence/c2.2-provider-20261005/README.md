# C2.2 intrinsic duration provider — local instance acceptance

Source `a2b8f6c89ed84839d8590e2b938afcd6d4f919fe` implements the frozen private
provider and full logical stream identity seam. Canonical conformance:24 passed,
zero failed/ignored/filtered, including all22 required named cases. Canonical
Rust1.99 and optional Rust1.88 library checks:61 passed,2 pre-existing
transport-input-dependent tests ignored. Strict all-target clippy and formatting
pass. No manifest, lock, RNG framing/generator, fixture or runner change.

The missing opaque-key payload found in initial source review is corrected and
covered by a distinct-identity/equal-duration regression. Debug redacts identifiers.
Rejection-then-counter-overflow preserves the original advancing owner. All raw
logs/command receipts are retained with hashes in acceptance.json. Baseline red
(native exit101, no behavioral tests) is retained separately. Independent review
and fresh coordinator canonical checks support local instance acceptance.

Hosted qualification and publication remain pending. C2.2 as a whole remains open:
policy admission permit, optional production bridge and complete mode/stream state
join remain required. C2.3 transit precedes the full C2.1 runtime join. No portable
checkpoint, calibrated empirical model, clinical acceptance or ED MVP claim.

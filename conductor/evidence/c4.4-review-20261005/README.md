# C4.4 retained development qualification

Compiled source: `79fac7ea52b8c21650759ef2d137104fa421c48d`. The publication commit adds evidence, the readback contract clarification and one exact historical Gitleaks fingerprint exception; its source relationship must be checked separately. This is synthetic implementation conformance, not clinical, public API or release acceptance.

## Executed gates

Rust 1.99.0 and explicitly pinned Rust 1.88.0 each passed the full calibration suite (146 passed, four named ignored), the explicit 42-case C4.1 candidate oracle gate, strict independent IPC/Parquet readback with 27 manifest/statistical mutation controls, and both actual supplemental diagnostic cases. Current all-target/all-feature Clippy passed. Independent PyArrow 25.0.1 readback regenerated exact Fraction oracles from inputs and verified counts/types/nulls/provenance; six mutations of actual outputs were rejected.

Numeric contracts remain C0/C4.1: W1 absolute tolerance 1e-12 times max(1, declared scale ticks), KS absolute tolerance 1e-12, exact identities and counts. All 42 candidate cases matched on both compilers. Small hand checks: [0,2] versus [1,3] has W1=1 and KS=1/2; [0] versus [0,0,0,0,900] has W1=180 and KS=1/5. Fractional [-1/2,1/2] versus [0,1/3] has W1=1/3 and KS=1/2. Opposite dependence retains marginal zero distance while its two conditional groups each have W1=KS=1.

Actual residual source: 15 raw instances (8 reference, 7 simulation), eight logical pair slots, three matched pairs, nine unmatched raw instances, and five eligible observations per side. Excluded=2, censored=1, missing=1, failed=1, infeasible=1 are overlapping diagnostics, not a partition. One causal reference resolves to one source event. Overlap/tie diagnostics independently reconcile raw 7/2 to eligible 2/2, ties 1/0 and all seven derived warnings. Empty/insufficient diagnostics retain declared zero/zero and one/zero groups.

## Retention and provenance

`qualification.tar.gz` contains normalized sorted USTAR members and deterministic gzip metadata, local command receipts/logs, actual IPC/Parquet and manifests, final candidate reports, independent verifier/results/negative controls, 76 committed source blobs, four original hosted ZIPs and selected extracted hosted evidence. `archive-manifest.json` binds every payload file; `archive-build.json` binds the compressed and canonical tar hashes. The archive also retains the earlier full-Clippy failure, distinctly named as superseded. Private lease tokens, credentials and build caches are excluded.

The hosted source snapshot is at exact compiled head: 47 successful checks, two intentional skips and one false-positive Gitleaks failure. The native owner run 37258123273 succeeded, including Linux/macOS current and Rust 1.88 feature combinations. Fresh hosted results for the publication head must be retained before parent acceptance; the archived source snapshot does not assert that later check succeeded.

## Historical scan disposition

The only exception is the exact fingerprint in `.gitleaksignore`: a historical error message for a duplicate raw join key. Static review found no credential or secret. Gitleaks 8.30.1 passed the reviewed history with that exact fingerprint and still detected the same diagnostic in a new occurrence. Managed security evidence is retained outside this repository in Codex Security's collection for this worktree: findings/c44-gitleaks-triage.md SHA-256 d7d58a455ade185b7e0912e8f630215da6f99aebf549e68b83f29e7a7af15561; artifacts/03_validation/c44-gitleaks/validation-receipts.json SHA-256 8082e20b6589e0fad064a9507773d1990674ea1ed8c2a363a1b8cadb4dfde36b. The exception does not cover other locations, future occurrences or dependency alert 69.

## Limits and coordination

C2 provider work and Track49 remain separate. Physical/logical Arrow and event schemas are unchanged. No p-values, uncertainty intervals, survival correction, clinical cutoffs, API stability or release approval are asserted. Website dependency alert 69 remains open.

The ignored prior output directory was renamed to retain the failed attempt; that rename exceeded the narrow original output-prefix claim but preserved all source and evidence. Parent preparation initialized both recorded submodule checkouts under a narrower declared initialization path; both pins remained unchanged. These coordination deviations are recorded, not represented as clean execution authority. Source input-hash rejection was recovered through a fresh clean worktree and reviewed transfer. Future Python verification disables bytecode writes.

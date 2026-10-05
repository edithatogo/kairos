# C2.2 fidelity export evidence

This directory preserves the external production-module smoke and local compatibility checks for the bounded `fidelity_export` instance. The commands ran at source commit `d926dea70199bf9e8a8d53a275376e8edb7f7934`. The reviewed current source is `c6eb105119ac72e3f1425993268d76ab6592640e`, which changes only rustfmt wrapping in one assertion; no native rerun was made after that formatting-only commit.

The Rust 1.76 and Rust 1.99 package tests each ran the 10 mode cases, 3 lineage cases, and 1 external Flow smoke exactly once. Rust 1.99 strict all-target Clippy passed with warnings denied. Raw outputs and the original result receipt are copied unchanged; `acceptance.json` records their hashes.

This evidence covers the public fidelity module export and its real Flow smoke only. It does not claim the borrowing permit, full policy acceptance, C2.2 runtime acceptance, hosted exact-head CI, or publication.

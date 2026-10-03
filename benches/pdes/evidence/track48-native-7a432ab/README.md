# Explicit pinned native evidence

Source `7a432ab46c3bfa5f3917f1cce0f4545fac39f649`. Rust1.98.1 Cargo/rustc and matching LLVM tools are explicitly bound; Cargo cache independently records the actual compiler. `just ci` passed 458 tests, 92.59% core line coverage, formatting, Clippy, rustdoc, cargo-deny and cargo-audit. Supplemental collector9/manifest/phase/DAG/clean-Git gates passed at the same source. Copies preserve raw bytes, log hashes and original execution paths.

Earlier wrapper attempts failed LLVM discovery/profile merging and do not constitute pinned workspace passes; their receipts/logs remain under `artifacts/track48-final-validation/`. Prior nominal1.98/1.76 behavioral receipts used actual Homebrew1.99 and are superseded for compiler provenance. This evidence does not establish hosted, distributed, Windows, HPC or package security acceptance.

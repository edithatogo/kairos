# Codec bridge native evidence

Scope: accepted additive local preview API; independent held-out acceptance and hosted/new-PR checks remain pending. Track48 remains In Progress; no transport, durable admission, authority fencing or distributed GVT claim.

The implementation worker receipt records base a0ca742 plus exact final source hashes, independently verified against commit2dbdd1b. Root integrated those exact two blobs at c19fbf4. Matching explicit Rust1.98.1 worker test/Clippy/format final commands pass, 98 tests; earlier fixture type/format failures remain unchanged separate attempts. Source implementation changes only checked reconstruction/inspection methods, leaving scheduling/rollback/GVT paths unchanged.

Root matching absolute Cargo/rustc/rustdoc1.76.0 test at c19fbf4 exits0 with98 tests and zero runnable doc examples. All crate/lock input hashes, compiler binary hashes, log and actual Cargo compiler-cache hashes are retained. Later docs-only commits do not retrospectively change that executed source identity. Inputs are deterministic source fixtures, including literal model RNG initialization; no external stochastic runner/seed override was provided.

Directories contain unchanged copies of executed receipts/logs/cache and hash manifests. This evidence leaf exceeds the normal five-file sizing default solely to preserve inseparable raw provenance and prior failed attempts; it changes no implementation or gate. No lease tokens/context snapshots are included.

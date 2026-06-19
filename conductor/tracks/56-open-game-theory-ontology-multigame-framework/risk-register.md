# Track 56 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| External ontology schemas are unstable or inconsistent | Parser churn and invalid generated components | Version schemas, store provenance, and require deterministic normalization fixtures |
| JSON-LD contexts do not match Turtle semantics | Silent ontology divergence | Require paired Turtle/JSON-LD equivalence tests before code generation |
| Generated Rust structs become an unreviewed public API | Compatibility breakage | Gate generated output through Track 25 API review before binding exposure |
| Graph topology uses pointer-like object graphs | Cache locality loss and borrow checker complexity | Require `EntityId` edge components and static scans for raw pointers, `Rc`, `Arc`, and boxed node graphs |
| `graph-relations` leaks into default builds | Core engine surface grows without explicit opt-in | Add default-build and `--no-default-features` compile tests |
| Extensive-form traversal cycles indefinitely | Solver hang or stack overflow | Add cycle/depth guards and deterministic error reporting |
| Normal-form and extensive-form solvers diverge on shared fixtures | Incorrect game semantics | Add cross-mode fixtures and conformance checks |
| Subrepository tracking is ambiguous | Source-of-truth confusion | Define subrepo tracking method, commit provenance, and handoff boundaries before ingestion |

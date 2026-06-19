# Track 58 Risk Register

| Risk | Impact | Mitigation |
|---|---|---|
| Non-deterministic generation | Noisy review and broken reproducibility | Sort all ontology inputs and generated symbols. |
| Generated API leaks unstable ontology terms | Downstream compatibility churn | API governance review before In Review. |
| Codegen bypasses Rust ownership rules | Unsafe generated code | Forbid unsafe and raw pointers in generated output. |

# Track 58 Risk Register

Severity scoring scale: Low/Medium/High/Critical = 1-4. Low = bounded documentation or local workflow inconvenience; Medium = delayed phase closeout or limited user-facing claim risk; High = broken implementation, release-blocking evidence gap, or unsafe public claim; Critical = data race, unsafe memory behavior, credential exposure, or knowingly false release/parity claim.

| Risk | Impact | Mitigation |
|---|---|---|
| Non-deterministic generation | Noisy review and broken reproducibility | Sort all ontology inputs and generated symbols. |
| Generated API leaks unstable ontology terms | Downstream compatibility churn | API governance review before In Review. |
| Codegen bypasses Rust ownership rules | Unsafe generated code | Forbid unsafe and raw pointers in generated output. |

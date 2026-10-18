# Track 62 Risk Register

Severity scoring scale: Low/Medium/High/Critical = 1-4. Low = bounded documentation or local workflow inconvenience; Medium = delayed phase closeout or limited user-facing claim risk; High = broken implementation, release-blocking evidence gap, or unsafe public claim; Critical = data race, unsafe memory behavior, credential exposure, or knowingly false release/parity claim.

| Risk | Likelihood | Impact | Severity | Mitigation |
|---|---|---|---|---|
| Upstream dependency churn or MSRV/license mismatch | Medium (2) | High (3) | High (3) | Pin compatible versions in `Cargo.toml`, validate with `cargo deny`, and gate behind explicit Cargo features. |
| Cross-language FFI or memory alignment bugs | Medium (2) | Critical (4) | High (3) | Use standard Arrow C Data Interface / PyO3 / wasm-bindgen safe wrappers and test under AddressSanitizer/Miri where supported. |
| Overclaiming performance before benchmark receipts exist | Low (1) | High (3) | Medium (2) | Keep track status `Planned` or `In Progress` until reproducible benchmark and conformance gates pass. |

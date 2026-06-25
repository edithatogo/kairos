# Track 60 Risk Register

Severity scoring scale: Low/Medium/High/Critical = 1-4. Low = bounded documentation or local workflow inconvenience; Medium = delayed phase closeout or limited user-facing claim risk; High = broken implementation, release-blocking evidence gap, or unsafe public claim; Critical = data race, unsafe memory behavior, credential exposure, or knowingly false release/parity claim.

| Risk | Impact | Mitigation |
|---|---|---|
| Solver semantics are under-specified | Non-reproducible results | Golden fixtures with explicit tie-breaking. |
| Matrix layout is not cache-local | Slow multi-game execution | Flat storage benchmarks and layout review. |
| Game-theory terminology diverges from ontology | API mismatch | Depend on Track 58 generated component review. |

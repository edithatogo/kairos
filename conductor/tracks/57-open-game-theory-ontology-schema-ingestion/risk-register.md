# Track 57 Risk Register

Severity scoring scale: Low/Medium/High/Critical = 1-4. Low = bounded documentation or local workflow inconvenience; Medium = delayed phase closeout or limited user-facing claim risk; High = broken implementation, release-blocking evidence gap, or unsafe public claim; Critical = data race, unsafe memory behavior, credential exposure, or knowingly false release/parity claim.

| Risk | Impact | Mitigation |
|---|---|---|
| External research schemas drift | Parser instability | Keep fixtures versioned and source-attributed. |
| Turtle/JSON-LD parsing becomes string scraping | Incorrect ontology semantics | Use structured parsers or a constrained grammar with negative fixtures. |
| Subrepo boundary is ambiguous | Dirty nested repo state | Record whether the subrepo is nested Git or repo directory before implementation. |
